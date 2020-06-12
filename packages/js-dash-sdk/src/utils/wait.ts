/**
 * Asynchronously wait for a specified number of milliseconds.
 *
 * @param {number} ms - Number of milliseconds to wait.
 *
 * @returns {Promise<void>} The promise to await on.
 */
export async function wait(ms): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
