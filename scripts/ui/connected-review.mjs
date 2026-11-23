import assert from "node:assert/strict";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;

// Host callbacks supply a controlled account presentation. All tested commands
// still go through actual native controls; no sign-in or service request occurs.
export async function verifyConnectedSettings(ui) {
  const { page, resize, snapshot, control, clickControl, capture } = ui;
  await page.evaluate("todoApp.cloudCapabilities(true,true,true); todoApp.cloudState(true,'Integration review account');");
  try {
    for (const [width, label] of [[1280,"wide"],[320,"narrow"]]) {
      await resize(width,844);
      await clickControl(button("Settings"),"connected Settings");
      for (const name of ["Sync now","Switch Microsoft account","Open local workspace","Connect Outlook categories","Sync device settings with OneDrive","Open Microsoft To Do","Import Outlook PST…","Export Outlook PST"]) {
        for(let attempt=0;attempt<12;attempt++) {
          if((await snapshot()).elements.some(element=>element.hit&&button(name)(element)))break;
          await page.wheel(width-60,650,0,280);
        }
        const element=await control(button(name),name);
        assert(element.x>=-1&&element.x+element.width<=width+1,`${name} overflows ${width}px`);
        assert(element.height>=36,`${name} retains a usable touch target`);
        if(name==="Sync device settings with OneDrive")await capture(`connected-services-${label}`);
      }
      await capture(`connected-data-${label}`);
      await clickControl(button("Close settings"),"close connected Settings");
    }
  } finally {
    await page.evaluate("todoApp.cloudState(false,'Local account'); todoApp.cloudCapabilities(true,false,false);");
  }
}
