const url = require('url');

const URI = {
  parse(uriString) {
    const {
      hash,
      protocol,
      pathname,
      port,
      hostname,
    } = url.parse(uriString);

    return {
      scheme: protocol,
      host: hostname,
      port: port || null,
      path: pathname,
      fragment: hash ? hash.substring(1) : null,
    };
  },

  serialize(components) {
    const {
      scheme,
      host,
      port,
      path,
      fragment,
    } = components;

    return url.format({
      host: port ? `${host}:${port}` : host,
      hash: fragment ? `#${fragment}` : undefined,
      protocol: scheme,
      pathname: path,
      port,
      hostname: host,
    });
  },

  resolve(baseURI, relativeURI) {
    return url.resolve(baseURI, relativeURI);
  },
};

module.exports = URI;
