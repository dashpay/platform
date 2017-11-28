/**
 * @typedef {Object} Config
 * @property {string} name
 * @property {string} insightUri
 * @property {Object} server
 * @property {boolean} server.enable
 * @property {number} server.port
 * @property {Object} node
 * @property {string} node.pubKey
 * @property {number} node.rep.port
 * @property {number} node.pub.port
 */

const configs = {
  regtest: {
    name: 'regtest',
    insightUri: 'http://127.0.0.1:3001/insight-api-dash',
    livenet: false,
    server: {
      enable: true,
      port: 3000,
    },
    node: {
      pubKey: 'XkifrWK9yHVzXLgeAaqjhjDJuFad6b',
      rep: {
        port: 40000,
      },
      pub: {
        port: 50000,
      },
    },
  },
  testnet: {
    name: 'testnet',
    insightUri: 'http://dev-test.insight.dashevo.org/insight-api-dash',
    livenet: false,
    server: {
      enable: true,
      port: 3000,
    },
    node: {
      pubKey: 'XkifrWK9yHVzXLgeAaqjhjDJuFad6b',
      rep: {
        port: 40000,
      },
      pub: {
        port: 50000,
      },
    },
  },
  livenet: {
    name: 'livenet',
    insightUri: 'http://insight.dashevo.org/insight-api-dash',
    livenet: false,
    server: {
      enable: true,
      port: 3000,
    },
    node: {
      pubKey: 'XkifrWK9yHVzXLgeAaqjhjDJuFad6b',
      rep: {
        port: 40000,
      },
      pub: {
        port: 50000,
      },
    },
  },
};

/**
 * @returns {Config}
 */
function getConfig() {
  const env = process.env.NODE_ENV;
  if (Object.keys(configs).indexOf(env) > -1) {
    return configs[env];
  }
  return configs.testnet;
}

/**
 * @type {Config}
 */
module.exports = getConfig();
