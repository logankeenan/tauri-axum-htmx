let localAppRequestCommand = "local_app_request";

export function initialize(initialPath, localAppRequestCommandOverride) {
  if (localAppRequestCommandOverride) {
    localAppRequestCommand = localAppRequestCommandOverride;
  }

  proxyFetch();
  window.addEventListener("DOMContentLoaded", async () => {
    const response = await window.fetch(initialPath);
    document.documentElement.innerHTML = await response.text();

    htmx.process(document.documentElement);
  });
}

function proxyFetch() {
  const originalFetch = window.fetch;

  window.fetch = async function (...args) {
    const [url, options] = args;
    if (url.startsWith("ipc://")) {
      return originalFetch(...args);
    }

    const request = {
      uri: url,
      method: options?.method || "GET",
      headers: options?.headers || {},
      ...(options?.body && { body: options.body }),
    };
    let response = await invoke(localAppRequestCommand, {
      localRequest: request,
    });

    while ([301, 302, 303, 307, 308].includes(parseInt(response.status_code))) {
      const location = response.headers["location"];

      const redirectRequest = {
        uri: location,
        method: "GET",
        headers: {},
      };
      response = await invoke("local_app_request", {
        localRequest: redirectRequest,
      });
    }

    const bodyByteArray = new Uint8Array(response.body);
    const decoder = new TextDecoder("utf-8");
    const bodyText = decoder.decode(bodyByteArray);

    const status = parseInt(response.status_code);
    const headers = new Headers(response.headers);
    return new Response(bodyText, { status, headers });
  };
}

// BEGIN XHR-FETCH-PROXY
(function (originalXMLHttpRequest) {
  class EventTarget {
    constructor() {
      this.eventListeners = {};
    }

    addEventListener(event, callback) {
      if (!this.eventListeners[event]) {
        this.eventListeners[event] = [];
      }
      this.eventListeners[event].push(callback);
    }

    removeEventListener(event, callback) {
      if (!this.eventListeners[event]) return;
      const index = this.eventListeners[event].indexOf(callback);
      if (index !== -1) {
        this.eventListeners[event].splice(index, 1);
      }
    }

    _triggerEvent(event, ...args) {
      if (this.eventListeners[event]) {
        this.eventListeners[event].forEach((callback) =>
          callback.apply(this, args),
        );
      }
    }
  }

  class ProxyXMLHttpRequest extends EventTarget {
    constructor() {
      super();
      this.onload = null;
      this.onerror = null;
      this.onreadystatechange = null;

      this.readyState = 0;
      this.status = 0;
      this.statusText = "";
      this.response = null;
      this.res10ponseText = null;
      this.responseType = "";
      this.responseURL = "";
      this.method = null;
      this.url = null;
      this.async = true;
      this.requestHeaders = {};
      this.controller = new AbortController(); // to handle aborts
      this.eventListeners = {};
      this.upload = new EventTarget(); // Adding upload event listeners
      this._triggerEvent("readystatechange");
    }

    open(method, url, async = true, user = null, password = null) {
      this.method = method;
      this.url = url;
      this.async = async;
      this.user = user;
      this.password = password;
      this.readyState = 1;
      this._triggerEvent("readystatechange");
    }

    send(data = null) {
      const options = {
        method: this.method,
        headers: this.requestHeaders,
        body: data,
        signal: this.controller.signal,
        mode: "cors",
        credentials: this.user || this.password ? "include" : "same-origin",
      };

      if (this.user && this.password) {
        const base64Credentials = btoa(`${this.user}:${this.password}`);
        options.headers["Authorization"] = `Basic ${base64Credentials}`;
      }
      this.readyState = 2;
      this._triggerEvent("readystatechange");
      fetch(this.url, options)
        .then((response) => {
          this.status = response.status;
          this.statusText = response.statusText;
          this.responseURL = response.url;
          this._parseHeaders(response.headers);

          this.readyState = 3;
          this._triggerEvent("readystatechange");
          return this._parseResponse(response);
        })
        .then((responseData) => {
          this.readyState = 4;
          this.response = responseData;
          this.responseText =
            typeof responseData === "string"
              ? responseData
              : JSON.stringify(responseData);
          this._triggerEvent("readystatechange");
          if (this.onload) this.onload();
        })
        .catch((error) => {
          if (this.onerror) this.onerror(error);
        });
    }

    setRequestHeader(header, value) {
      this.requestHeaders[header] = value;
    }

    abort() {
      this.controller.abort();
      this.readyState = 0;
      this._triggerEvent("readystatechange");
    }

    getResponseHeader(header) {
      return this.responseHeaders[header.toLowerCase()] || null;
    }

    getAllResponseHeaders() {
      return Object.entries(this.responseHeaders)
        .map(([key, value]) => `${key}: ${value}`)
        .join("\r\n");
    }

    overrideMimeType(mime) {
      this.overrideMime = mime;
    }

    _parseHeaders(headers) {
      this.responseHeaders = {};
      headers.forEach((value, key) => {
        this.responseHeaders[key.toLowerCase()] = value;
      });
    }

    _parseResponse(response) {
      const contentType = response.headers.get("content-type");
      if (contentType && contentType.includes("application/json")) {
        return response.json();
      } else if (
        contentType &&
        (contentType.includes("text/") || contentType.includes("xml"))
      ) {
        return response.text();
      } else {
        return response.blob(); // default to blob for binary data
      }
    }

    _triggerEvent(event, ...args) {
      super._triggerEvent(event, ...args);
      if (this[`on${event}`]) {
        this[`on${event}`].apply(this, args);
      }

      if (
        event.startsWith("progress") ||
        event === "loadstart" ||
        event === "loadend" ||
        event === "abort"
      ) {
        this.upload._triggerEvent(event, ...args);
      }
    }
  }

  window.XMLHttpRequest = ProxyXMLHttpRequest;
})(window.XMLHttpRequest);
// END XHR-FETCH-PROXY
