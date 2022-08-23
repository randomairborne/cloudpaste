onmessage = (event) => {
  importScripts('//cdnjs.cloudflare.com/ajax/libs/highlight.js/11.6.0/highlight.min.js');
  const result = self.hljs.highlightAuto(event.data);
  postMessage(result.value);
};
