export default async function getMetrics(host, port) {
  try {
    const response = await fetch(`http://${host}:${port}/metrics`);

    if (response.status === 200) {
      return response.text();
    }

    if (process.env.DEBUG) {
      console.error(`Unknown response ${response.status} from the metrics`);
    }

    return null;
  } catch (e) {
    if (process.env.DEBUG) {
      console.error(e);
    }

    return null;
  }
}
