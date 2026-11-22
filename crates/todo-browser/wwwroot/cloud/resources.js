const text = value => typeof value === "string" ? value : "";

export function sourceLinks(task) {
  return (task.linkedResources || []).filter(link => link.applicationName !== "FerroUI To Do").map(link => {
    let web_url = text(link.webUrl);
    try {
      const url = new URL(web_url);
      if (!["http:", "https:"].includes(url.protocol) || /[\u0000-\u001f\u007f]/.test(web_url) || url.username || url.password) web_url = "";
    } catch { web_url = ""; }
    return { id: text(link.id), application_name: text(link.applicationName), display_name: text(link.displayName), web_url, external_id: text(link.externalId) };
  });
}

export function cloudList(list) {
  return { is_owner: list.isOwner !== false, is_shared: !!list.isShared, well_known_name: list.wellknownListName || "none" };
}
