export async function requestNotifications() {
  if (!("Notification" in window)) throw new Error("This browser does not support reminders. On iPhone, install To Do on your Home Screen first.");
  const permission = Notification.permission === "granted" ? "granted" : await Notification.requestPermission();
  if (permission === "denied") throw new Error("Notifications are blocked. Allow them in this site's browser settings to receive reminders.");
  if (permission !== "granted") throw new Error("Notification permission was not granted. Choose Enable notifications to try again.");
}

export async function notify(title, body, delivery = {}) {
  if (!("Notification" in window) || Notification.permission !== "granted") return false;
  const current = new URL(location.href), target = new URL(".", current);
  const demo = current.searchParams.get("demo") === "1";
  if (demo) target.searchParams.set("demo", "1");
  // Keep only the document mode. OAuth callback queries and hashes must never
  // become persistent notification data or be replayed when opening the app.
  const scope = new URL(".", target);
  const registration = await navigator.serviceWorker?.getRegistration(scope.href);
  if (delivery.shouldShow && !delivery.shouldShow()) return false;
  const options = { body, icon: new URL("icon.svg", scope).href,
    tag: delivery.tag || `todo-${demo ? "demo" : "personal"}-${title}`, renotify: false, data: { url: target.href } };
  if (registration?.scope === scope.href && registration.active) {
    await registration.showNotification(delivery.getTitle?.() || title, options);
  } else {
    const notification = new Notification(delivery.getTitle?.() || title, options);
    notification.onclick = () => { notification.close(); window.focus(); };
  }
  return true;
}

let audio;
export async function completionSound() {
  const AudioContext = window.AudioContext || window.webkitAudioContext;
  if (!AudioContext) return;
  audio ||= new AudioContext();
  if (audio.state === "suspended") await audio.resume();
  const now = audio.currentTime;
  for (const [frequency, start] of [[523.25, 0], [783.99, .065]]) {
    const oscillator = audio.createOscillator();
    const gain = audio.createGain();
    oscillator.type = "sine";
    oscillator.frequency.value = frequency;
    gain.gain.setValueAtTime(0, now + start);
    gain.gain.linearRampToValueAtTime(.055, now + start + .008);
    gain.gain.exponentialRampToValueAtTime(.001, now + start + .18);
    oscillator.connect(gain).connect(audio.destination);
    oscillator.start(now + start);
    oscillator.stop(now + start + .2);
  }
}
