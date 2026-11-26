import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import configJsonSchema from '../../../src/config/configJsonSchema.js';
import obfuscateConfig, { SECRET_FIELD_NAMES } from '../../../src/config/obfuscateConfig.js';

/**
 * @param {*} node - a JSON schema, or any part of one
 * @param {Set<string>} names - collects every property name declared in it
 */
function collectPropertyNames(node, names) {
  if (typeof node !== 'object' || node === null) {
    return;
  }

  if (typeof node.properties === 'object' && node.properties !== null) {
    Object.keys(node.properties).forEach((name) => names.add(name));
  }

  Object.values(node).forEach((child) => collectPropertyNames(child, names));
}

describe('obfuscateConfig', () => {
  // Secrets are recognised by the name of the field they are stored in, both
  // in the config copy a doctor report carries and in every other file of it.
  // A field added under a name missing from the list would be shared in clear
  // with nothing failing, so every name that reads like a secret has to be
  // either on the list or cleared here with a reason.
  it('should treat every config field named like a secret as a secret', () => {
    const notSecret = {
      seeds: 'addresses of the peers a node connects to first',
    };

    const names = new Set();
    collectPropertyNames(configJsonSchema, names);

    const unclassified = [...names]
      .filter((name) => /pass|secret|token|key|seed|mnemonic|wif|cred|auth/i.test(name))
      .filter((name) => !SECRET_FIELD_NAMES.includes(name) && !Object.hasOwn(notSecret, name));

    expect(unclassified).to.be.empty();
  });

  it('should hide the value of every secret field', () => {
    const config = getBaseConfigFactory()();

    const secrets = {
      'core.rpc.users.dapi.password': 'dapiRpcPassword7Qx',
      'core.tor.control.password': 'torControlPassword7Qx',
      'core.spork.privateKey': 'sporkPrivateKey7Qx',
      'core.masternode.operator.privateKey': 'operatorPrivateKey7Qx',
      'platform.gateway.ssl.providerConfigs.zerossl.apiKey': 'zeroSslApiKey'.padEnd(32, '7'),
      'platform.drive.tenderdash.node.key': 'tenderdashNodeKey7Qx',
    };

    Object.entries(secrets).forEach(([path, value]) => config.set(path, value));

    const obfuscated = obfuscateConfig(config);

    Object.entries(secrets).forEach(([path, value]) => {
      expect(obfuscated.get(path)).to.equal('*'.repeat(value.length));
    });
  });
});
