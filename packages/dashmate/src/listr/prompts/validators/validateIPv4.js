function validateIPv4(ip) {
  return Boolean(ip.match(/^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/));
}

module.exports = validateIPv4;
