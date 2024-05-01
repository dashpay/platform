/**
 * @param {string} value
 * @returns {boolean}
 */
export default function validatePercentage(value) {
  const reminder = value.split('.')[1];

  return Number(value) >= 0 && Number(value) <= 100 && (!reminder || reminder.length <= 2);
}
