export default async function fetchHTTP(url, method, data) {
  const options = { method };
  if (method === 'POST') {
    options.headers = { 'Content-Type': 'application/json' };
    options.body = JSON.stringify(data);
  }

  const response = await fetch(url, options);

  if (response.status === 200) {
    return response.text();
  }

  throw new Error(`Unknown response ${response.status} from ${url} ${method} ${JSON.stringify(data)}`);
}
