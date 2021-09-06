function hasProp(obj, prop) {
  if (!obj) return false;
  if (Array.isArray(obj)) {
    return obj.includes(prop);
  }
  return {}.hasOwnProperty.call(obj, prop);
}

module.exports = hasProp;
