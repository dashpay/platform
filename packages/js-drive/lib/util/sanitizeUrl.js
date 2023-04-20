function sanitizeUrl(url) {
  for (let i = 0, len = url.length; i < len; i++) {
    const charCode = url.charCodeAt(i);
    // Some systems do not follow RFC and separate the path and query
    // string with a `;` character (code 59), e.g. `/foo;jsessionid=123456`.
    // Thus, we need to split on `;` as well as `?` and `#`.
    if (charCode === 63 || charCode === 59 || charCode === 35) {
      return url.slice(0, i);
    }
  }

  return url;
}

module.exports = sanitizeUrl;
