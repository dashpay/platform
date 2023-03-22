function validatePercentage(value) {
  const reminder = value.split('.')[1];

  return Number(value) <= 100 && (!reminder || reminder.length <= 2);
}

module.exports = validatePercentage;
