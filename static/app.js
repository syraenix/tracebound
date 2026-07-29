(() => {
  let traceSource;

  function connectTrace() {
    if (traceSource) {
      traceSource.close();
      traceSource = undefined;
    }
    const trace = document.querySelector("[data-trace-url]");
    if (!trace || !window.EventSource) return;

    traceSource = new EventSource(trace.dataset.traceUrl);
    traceSource.addEventListener("trace", (message) => {
      const event = JSON.parse(message.data);
      if (trace.querySelector(`[data-sequence="${event.sequence}"]`)) return;

      const item = document.createElement("li");
      item.dataset.sequence = event.sequence;
      const metadata = document.createElement("span");
      metadata.className = "meta";
      metadata.textContent = `#${event.sequence} · ${event.actor}`;
      item.append(metadata, document.createElement("br"), event.summary);
      trace.append(item);
    });
  }

  document.addEventListener("DOMContentLoaded", connectTrace);
  document.addEventListener("htmx:afterSwap", connectTrace);
})();
