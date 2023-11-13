/**
 * @param {string} dateString
 * @returns {Date}
 */
export function convertDate(dateString) {
  const parts = dateString.split(/[- :]/);
  return new Date(parts[0], parts[1] - 1, parts[2], parts[3], parts[4], parts[5]);
}
