import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import configJsonSchema from '../../../src/config/configJsonSchema.js';
import obfuscateConfig, {
  getConfigSecretValues,
  SECRET_FIELD_NAMES,
} from '../../../src/config/obfuscateConfig.js';

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

  // A URL can carry its own credentials, whatever the field holding it is
  // called. The rest of the URL says where the node is pointed and stays.
  describe('credentials embedded in a URL', () => {
    const urlPath = 'platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl';

    /**
     * @param {string} url
     * @return {{config: Config, obfuscatedUrl: string}}
     */
    function obfuscateUrl(url) {
      const config = getBaseConfigFactory()();

      config.set(urlPath, url);

      return { config, obfuscatedUrl: obfuscateConfig(config).get(urlPath) };
    }

    it('should hide them and keep where the URL points', () => {
      const { config, obfuscatedUrl } = obfuscateUrl(
        'https://acmeUser:acmePassword7Qx@acme.example.com/directory',
      );

      expect(obfuscatedUrl).to.equal('https://********:***************@acme.example.com/directory');
      expect(getConfigSecretValues(config)).to.include.members([
        'acmeUser:acmePassword7Qx',
        'acmePassword7Qx',
      ]);
    });

    // URL parsers take the last `@` before the host as the separator, so a
    // password can hold one unescaped
    it('should hide all of a password that holds an @', () => {
      const { config, obfuscatedUrl } = obfuscateUrl(
        'https://acmeUser:acme@Password7Qx@acme.example.com/directory',
      );

      expect(obfuscatedUrl).to.not.contain('Password7Qx');
      expect(obfuscatedUrl).to.match(/@acme\.example\.com\/directory$/);
      expect(getConfigSecretValues(config)).to.include('acme@Password7Qx');
    });

    it('should hide the credentials of a URL inside another URL', () => {
      const { config, obfuscatedUrl } = obfuscateUrl(
        'https://acmeUser:acmePassword7Qx@acme.example.com/directory'
          + '?next=https://innerUser:innerPassword7Qx@inner.example.com/',
      );

      expect(obfuscatedUrl).to.not.contain('acmePassword7Qx');
      expect(obfuscatedUrl).to.not.contain('innerPassword7Qx');
      expect(obfuscatedUrl).to.contain('@inner.example.com/');
      expect(getConfigSecretValues(config)).to.include('innerPassword7Qx');
    });

    // A service logs the password as it uses it, not as the URL escapes it
    it('should find the password of a URL inside other text as written and decoded', () => {
      const config = getBaseConfigFactory()();

      config.set('core.docker.commandArgs', [
        '-proxy=http://proxyUser:proxy%40Password7Qx@proxy.example.com:3128',
      ]);

      expect(obfuscateConfig(config).get('core.docker.commandArgs')[0]).to.not.contain('Password7Qx');
      expect(getConfigSecretValues(config)).to.include.members([
        'proxy%40Password7Qx',
        'proxy@Password7Qx',
      ]);
    });

    // A token is often given as the user name alone, and a URL parser also
    // takes it without the slashes after the scheme
    it('should find a token given as the user name of a URL', () => {
      const config = getBaseConfigFactory()();

      config.set('platform.drive.abci.docker.build.buildArgs', {
        REGISTRY_URL: 'https:registryToken7Qx@registry.example.com/',
      });

      expect(getConfigSecretValues(config)).to.include('registryToken7Qx');
    });

    // A service can print any part of the credentials on its own, as it uses
    // them rather than as the URL escapes them
    it('should find each part of the credentials, as written and decoded', () => {
      const { config } = obfuscateUrl(
        'https://acme%2BUser7Qx:acme%40Password7Qx@acme.example.com/directory',
      );

      expect(getConfigSecretValues(config)).to.include.members([
        'acme%2BUser7Qx:acme%40Password7Qx',
        'acme+User7Qx:acme@Password7Qx',
        'acme%2BUser7Qx',
        'acme+User7Qx',
        'acme%40Password7Qx',
        'acme@Password7Qx',
      ]);
    });

    it('should find a token in a URL inside other text, as written and decoded', () => {
      const config = getBaseConfigFactory()();

      config.set('description', 'mirror https://mirror%40Token7Qx@mirror.example.com/ for builds');

      expect(getConfigSecretValues(config)).to.include.members([
        'mirror%40Token7Qx',
        'mirror@Token7Qx',
      ]);
    });

    it('should hide them when the URL has extra slashes before them', () => {
      const { config, obfuscatedUrl } = obfuscateUrl(
        'https:///acmeUser:acmePassword7Qx@acme.example.com/directory',
      );

      expect(obfuscatedUrl).to.not.contain('acmePassword7Qx');
      expect(obfuscatedUrl).to.contain('acme.example.com/directory');
      expect(getConfigSecretValues(config)).to.include('acmePassword7Qx');
    });
  });

  // Build arguments are free-form, and a proxy or registry credential is a
  // normal thing to pass through one. Only those known to select how an image
  // is built are shown.
  it('should hide every build argument not known to be harmless', () => {
    const config = getBaseConfigFactory()();

    config.set('platform.drive.abci.docker.build.buildArgs', {
      CARGO_BUILD_PROFILE: 'release',
      SDK_TEST_DATA: 'true',
      HTTPS_PROXY: 'http://proxyUser:proxyPassword7Qx@proxy.example.com:3128',
      REGISTRY_TOKEN: 'registryToken7Qx',
    });

    const buildArgs = obfuscateConfig(config).get('platform.drive.abci.docker.build.buildArgs');

    expect(buildArgs.CARGO_BUILD_PROFILE).to.equal('release');
    expect(buildArgs.SDK_TEST_DATA).to.equal('true');
    expect(buildArgs.HTTPS_PROXY).to.match(/^\*+$/);
    expect(buildArgs.REGISTRY_TOKEN).to.match(/^\*+$/);

    expect(getConfigSecretValues(config)).to.include.members([
      'http://proxyUser:proxyPassword7Qx@proxy.example.com:3128',
      // A log line can name the proxy password on its own
      'proxyPassword7Qx',
      'registryToken7Qx',
    ]);
    expect(getConfigSecretValues(config)).to.not.include('release');
  });

  // Core takes its options on the command line as well as from its config
  // file, and a key or a password is a normal thing to pass that way. The
  // option names stay: which options a node runs with is what makes the
  // config worth reading.
  it('should hide the value of every option passed to Core on its command line', () => {
    const config = getBaseConfigFactory()();
    const blsKey = '4a'.repeat(32);

    config.set('core.docker.commandArgs', [
      `-masternodeblsprivkey=${blsKey}`,
      '--rpcpassword=cliPassword7Qx',
      '-debug=net',
      '-printtoconsole',
    ]);

    expect(obfuscateConfig(config).get('core.docker.commandArgs')).to.deep.equal([
      `-masternodeblsprivkey=${'*'.repeat(blsKey.length)}`,
      '--rpcpassword=**************',
      '-debug=***',
      '-printtoconsole',
    ]);
    expect(getConfigSecretValues(config)).to.include.members([blsKey, 'cliPassword7Qx']);
  });

  // Core reads an option's value only after `=`, and refuses to start on a
  // bare argument, so one written as a separate argument is a value meant for
  // the option before it
  it('should hide an argument passed to Core that is not an option', () => {
    const config = getBaseConfigFactory()();

    config.set('core.docker.commandArgs', ['-rpcpassword', 'splitPassword7Qx', '-printtoconsole']);

    expect(obfuscateConfig(config).get('core.docker.commandArgs')).to.deep.equal([
      '-rpcpassword',
      '****************',
      '-printtoconsole',
    ]);
    expect(getConfigSecretValues(config)).to.include('splitPassword7Qx');
  });

  // Build arguments are recognised by where the schema puts them, not by name
  // alone: a Drive logger can be called anything, `buildArgs` included, and
  // its settings are constrained values the config has to keep.
  it('should leave a logger alone whatever it is named', () => {
    const config = getBaseConfigFactory()();

    const logger = {
      destination: 'stdout',
      level: 'info',
      format: 'full',
      color: null,
    };

    config.set('platform.drive.abci.logs.buildArgs', logger);

    expect(obfuscateConfig(config).get('platform.drive.abci.logs.buildArgs')).to.deep.equal(logger);
  });
});
