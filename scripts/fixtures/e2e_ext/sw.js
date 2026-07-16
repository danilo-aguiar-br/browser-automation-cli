chrome.runtime.onInstalled.addListener(() => {
  console.log("e2e-ext installed");
});
// Keep SW alive briefly for list/trigger
self.addEventListener("activate", () => {
  console.log("e2e-ext activate");
});
