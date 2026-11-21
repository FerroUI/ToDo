export async function requestNotifications() {
  if (!("Notification" in window)) throw new Error("This browser does not support reminders. On iPhone, install To Do on your Home Screen first.");
  const permission = await Notification.requestPermission();
  if (permission !== "granted") throw new Error("Notifications are blocked. Allow them in this site's browser settings to receive reminders.");
}

export async function notify(title, body) {
  if (!("Notification" in window) || Notification.permission !== "granted") return;
  const registration = await navigator.serviceWorker?.getRegistration();
  const options = { body, icon: "./icon.svg", tag: `todo-${title}`, data: { url: location.href } };
  if (registration) await registration.showNotification(title, options);
  else new Notification(title, options);
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
