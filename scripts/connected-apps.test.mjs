import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

const mocks={
  "auth.js":`const f=()=>globalThis.connectedFixture;export const accountKey=()=>f().account;export const serviceEnvironment=()=>({id:'global',categories:true,presentation:true});export const requestScopes=async scopes=>{f().scopes.push(scopes);if(f().consentError)throw f().consentError;};`,
  "categories.js":`const f=()=>globalThis.connectedFixture;export const CATEGORY_SCOPES=['MailboxSettings.Read'];export const cachedOutlookCategories=()=>[];export const forgetOutlookCategories=()=>f().forgot++;export const readOutlookCategories=async()=>{f().categoryReads++;if(f().categoryError)throw f().categoryError;return[{id:'one',name:'Work',color:'#0078D4',color_name:'Blue'}];};`,
  "presentation.js":`const f=()=>globalThis.connectedFixture;export const forgetPresentationMappings=()=>f().presentationForgot++;export const resetPresentationSession=()=>{};export const synchronizePresentation=async()=>{f().presentationReads++;if(f().presentationError)throw f().presentationError;return true;};`
};
registerHooks({resolve(specifier,context,next){const r=next(specifier,context),name=r.url.split('/cloud/')[1];return mocks[name]?{url:'fixture:connected/'+name,shortCircuit:true}:r;},load(url,context,next){if(!url.startsWith('fixture:connected/'))return next(url,context);return{format:'module',shortCircuit:true,source:mocks[url.split('/').at(-1)]};}});
const {connectedAppRequest,synchronizeConnectedApps,resetConnectedApps,connectedPreferencesChanged}=await import('../crates/todo-browser/wwwroot/cloud/connected.js');
function fixture(){
  const f={account:'account',settings:{},errors:[],scopes:[],categoryReads:0,presentationReads:0,forgot:0,presentationForgot:0,categories:[]};
  globalThis.connectedFixture=f;
  f.runtime={exportData:()=>JSON.stringify({settings:f.settings}),setIntegrationPreference:(key,value)=>f.settings[key]=value,
    outlookCategories:json=>f.categories=JSON.parse(json),hostError:message=>f.errors.push(message)};
  connectedPreferencesChanged();return f;
}

test('optional features request their individual permissions only after an explicit action',async()=>{
  const f=fixture();await synchronizeConnectedApps(f.runtime,f.account,f.settings);
  assert.equal(f.categoryReads,0);assert.equal(f.presentationReads,0);assert.equal(f.scopes.length,0);
  await connectedAppRequest(f.runtime,'enableOutlookCategories');assert.equal(f.settings.outlook_categories_enabled,true);
  await connectedAppRequest(f.runtime,'enablePresentationSync');assert.equal(f.settings.presentation_sync_enabled,true);
  assert.deepEqual(f.scopes,[['MailboxSettings.Read'],['Files.ReadWrite.AppFolder']]);
});

test('rejected consent restores the previous preference without modifying task sync settings',async()=>{
  const f=fixture();f.settings.auto_sync=true;f.consentError=new Error('consent cancelled');
  await assert.rejects(connectedAppRequest(f.runtime,'enablePresentationSync'),/cancelled/);
  assert.equal(f.settings.presentation_sync_enabled,false);assert.equal(f.settings.auto_sync,true);
});

test('category writing requires its own explicit consent and disabling reading also disables writing',async()=>{
  const f=fixture();f.settings.outlook_categories_enabled=true;
  await connectedAppRequest(f.runtime,'enableOutlookCategoryEditing');
  assert.equal(f.settings.outlook_categories_write_enabled,true);
  assert.deepEqual(f.scopes,[['MailboxSettings.ReadWrite']]);
  await connectedAppRequest(f.runtime,'disableOutlookCategories');
  assert.equal(f.settings.outlook_categories_enabled,false);
  assert.equal(f.settings.outlook_categories_write_enabled,false);
  await assert.rejects(connectedAppRequest(f.runtime,{kind:'createOutlookCategory',name:'A',color:'preset1'}),/Enable Outlook category editing/);
});

test('declined category-write consent preserves existing read access',async()=>{
  const f=fixture();f.settings.outlook_categories_enabled=true;f.consentError=new Error('cancelled');
  await assert.rejects(connectedAppRequest(f.runtime,'enableOutlookCategoryEditing'),/cancelled/);
  assert.equal(f.settings.outlook_categories_enabled,true);assert.equal(f.settings.outlook_categories_write_enabled,false);
});

test('category consent failure pauses that service while presentation sync continues',async()=>{
  const f=fixture();f.settings={outlook_categories_enabled:true,presentation_sync_enabled:true};
  f.categoryError=Object.assign(new Error('consent required'),{requiresInteraction:true});
  const first=await synchronizeConnectedApps(f.runtime,f.account,f.settings);
  assert.equal(first.changed,true);assert.equal(first.issues.length,1);assert.equal(f.presentationReads,1);
  await synchronizeConnectedApps(f.runtime,f.account,f.settings);assert.equal(f.categoryReads,1);assert.equal(f.presentationReads,2);assert.equal(f.errors.length,1);
  f.categoryError=undefined;const retry=await synchronizeConnectedApps(f.runtime,f.account,f.settings,{force:true});
  assert.deepEqual(retry.issues,[]);assert.equal(f.categories[0].name,'Work');
});

test('presentation corruption does not discard a valid category catalogue',async()=>{
  const f=fixture();f.settings={outlook_categories_enabled:true,presentation_sync_enabled:true};f.presentationError=new Error('invalid replica');
  const result=await synchronizeConnectedApps(f.runtime,f.account,f.settings);
  assert.equal(f.categories[0].color,'#0078D4');assert.match(result.issues[0],/Device settings sync/);
  await synchronizeConnectedApps(f.runtime,f.account,f.settings);assert.equal(f.presentationReads,1);
});

test('category refresh can obtain missing optional consent and disconnect clears cached colors',async()=>{
  const f=fixture();f.categoryError=Object.assign(new Error('needs consent'),{requiresInteraction:true});
  await connectedAppRequest(f.runtime,'refreshOutlookCategories');assert.deepEqual(f.scopes,[['MailboxSettings.Read']]);
  f.categories=[{name:'old'}];await connectedAppRequest(f.runtime,'disableOutlookCategories');
  assert.equal(f.settings.outlook_categories_enabled,false);assert.deepEqual(f.categories,[]);assert.equal(f.forgot,1);
});

test('restore forgets presentation bindings while ordinary disconnect preserves them',()=>{
  const f=fixture();resetConnectedApps(f.runtime);assert.equal(f.presentationForgot,0);
  resetConnectedApps(f.runtime,{restore:true});assert.equal(f.presentationForgot,1);
});
