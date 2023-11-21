/**
 * Asynchronously wait for a specified number of milliseconds.
 * @param {Number} ms - Number of milliseconds to wait.
 * @return {Promise<void>} The promise to await on.
 */
export default async (ms) => new Promise((resolve) => { setTimeout(resolve, ms); });
