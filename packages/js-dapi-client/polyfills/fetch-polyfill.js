/* eslint-disable */

const { default: fetch, Headers, Request, Response } = require('node-fetch');

if (typeof window === 'undefined') {
  globalThis.fetch = fetch;
  globalThis.Headers = Headers;
  globalThis.Request = Request;
  globalThis.Response = Response;
}
