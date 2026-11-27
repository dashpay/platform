import os from 'os';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import { getOperatorIdentity } from '../../../src/util/maskOperatorIdentity.js';
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
      'core.spork.privateKey': 'spork test fixture',
      'core.masternode.operator.privateKey': 'operator test fixture',
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
      expect(new URL(obfuscatedUrl).searchParams.get('next'))
        .to.equal('https://*********:****************@inner.example.com/');
      expect(getConfigSecretValues(config)).to.include('innerPassword7Qx');
    });

    // A URL passed as a query value is percent-encoded, so its credentials
    // only show once the value is decoded
    it('should hide the credentials of a URL encoded into a query value', () => {
      const inner = 'https://innerUser:innerPassword7Qx@inner.example.com/';
      const { config, obfuscatedUrl } = obfuscateUrl(
        `https://acme.example.com/directory?upstream=${encodeURIComponent(inner)}&mode=strict`,
      );

      expect(obfuscatedUrl).to.not.contain('innerPassword7Qx');
      expect(obfuscatedUrl).to.not.contain('innerUser');
      expect(new URL(obfuscatedUrl).searchParams.get('upstream'))
        .to.equal('https://*********:****************@inner.example.com/');
      expect(new URL(obfuscatedUrl).searchParams.get('mode')).to.equal('strict');
      expect(getConfigSecretValues(config)).to.include.members(['innerUser', 'innerPassword7Qx']);
    });

    // Decoded, a query value can hold a URL only a parser reads, such as one
    // with a space in its password
    it('should read a URL in a query value the way a URL parser does', () => {
      const inner = 'https://innerUser:inner Password7Qx@inner.example.com/';
      const { config, obfuscatedUrl } = obfuscateUrl(
        `https://acme.example.com/directory?upstream=${encodeURIComponent(inner)}`,
      );

      expect(decodeURIComponent(obfuscatedUrl)).to.not.contain('Password7Qx');
      expect(getConfigSecretValues(config)).to.include('inner Password7Qx');
    });

    // A log that prints the outer URL prints the inner one's credentials
    // encoded once more
    it('should find credentials from a query value as the outer URL writes them', () => {
      const inner = 'https://innerUser:inner%40Password7Qx@inner.example.com/';
      const { config } = obfuscateUrl(
        `https://acme.example.com/directory?upstream=${encodeURIComponent(inner)}`,
      );

      expect(getConfigSecretValues(config)).to.include.members([
        'inner%40Password7Qx',
        'inner@Password7Qx',
        'inner%2540Password7Qx',
      ]);
    });

    it('should leave a URL without credentials exactly as written', () => {
      const url = 'https://acme.example.com/directory?mode=strict&x=a%20b';

      expect(obfuscateUrl(url).obfuscatedUrl).to.equal(url);
    });

    // A query built with URLSearchParams escapes more than encodeURIComponent
    // does, `!` among it
    it('should find credentials from a query value as a form-encoded query writes them', () => {
      const inner = 'https://innerUser:long!Password7Qx@inner.example.com/';
      const { config } = obfuscateUrl(
        `https://acme.example.com/directory?${new URLSearchParams({ upstream: inner })}`,
      );

      expect(getConfigSecretValues(config)).to.include.members([
        'long!Password7Qx',
        'long%21Password7Qx',
      ]);
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

    // A URL parser also takes a web URL with no slashes after the scheme, so
    // a value can carry one that way inside other text too
    it('should hide a token in a web URL without slashes inside other text', () => {
      const config = getBaseConfigFactory()();

      config.set('core.docker.image', 'mirror https:mirrorToken7Qx@mirror.example.com/ for builds');

      expect(obfuscateConfig(config).get('core.docker.image'))
        .to.equal(`mirror https:${'*'.repeat('mirrorToken7Qx'.length)}@mirror.example.com/ for builds`);
      expect(getConfigSecretValues(config)).to.include('mirrorToken7Qx');
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

  // The config copy is shared like the rest of the report, and paths in it
  // carry the home directory too. They stay absolute, since the schema
  // requires that of log paths and doctor validates the copy when it reads a
  // report back.
  it('should mask the home directory in paths and keep them absolute', () => {
    const config = getBaseConfigFactory()();
    const { homePath } = getOperatorIdentity();

    config.set('core.log.filePath', `${homePath}/dash/core.log`);
    config.set('platform.drive.abci.logs.file', {
      destination: `${homePath}/dash/drive.log`,
      level: 'info',
      format: 'full',
      color: null,
    });

    const obfuscated = obfuscateConfig(config);

    expect(obfuscated.get('core.log.filePath')).to.equal('/~/dash/core.log');
    expect(obfuscated.get('platform.drive.abci.logs.file.destination')).to.equal('/~/dash/drive.log');
    expect(JSON.stringify(obfuscated)).to.not.contain(homePath);
  });

  it('should keep the config valid when HOME is the root of the file system', function it() {
    this.sinon.stub(os, 'homedir').returns('/');

    const config = getBaseConfigFactory()();

    config.set(
      'platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl',
      'https://acme.example.com/directory',
    );

    expect(obfuscateConfig(config).get('platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl'))
      .to.equal('https://acme.example.com/directory');
  });

  // Nothing reads the free text an operator writes as a value, and it can
  // quote secrets and links in any form, so the copy keeps only whether it
  // was set
  it('should leave the description and contact addresses out, keeping whether they were set', () => {
    const config = getBaseConfigFactory()();

    config.set('description', "built from https://u:p@builds.example.com/team's/drive.tar?sig=token by alice");
    config.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', 'alice@example.com');

    const obfuscated = obfuscateConfig(config);

    expect(obfuscated.get('description')).to.equal('<redacted>');
    expect(obfuscated.get('platform.gateway.ssl.providerConfigs.letsencrypt.email'))
      .to.equal('<redacted>');

    config.set('description', '');
    config.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', null);

    const unset = obfuscateConfig(config);

    expect(unset.get('description')).to.equal('');
    expect(unset.get('platform.gateway.ssl.providerConfigs.letsencrypt.email')).to.be.null();
  });

  // Free text is recognised where the schema puts it. A JSON access log
  // template can name its fields anything, and their values are Envoy's
  // format operators, which the copy has to keep.
  it('should keep an access log template field named like a free-text field', () => {
    const config = getBaseConfigFactory()();

    const accessLog = {
      type: 'stdout',
      format: 'json',
      template: { description: '%RESPONSE_CODE_DETAILS%', email: '%REQ(X-CONTACT)%' },
    };

    config.set('platform.gateway.log.accessLogs', [accessLog]);

    expect(obfuscateConfig(config).get('platform.gateway.log.accessLogs')).to.deep.equal([accessLog]);
  });

  // Dashmate keeps its files under the home directory HOME names, which can
  // differ from the one the account is registered with; both name the
  // operator
  it('should mask both the registered home directory and the one HOME names', function it() {
    this.sinon.stub(os, 'userInfo').returns({ username: 'operator', homedir: '/home/operator' });
    this.sinon.stub(os, 'homedir').returns('/srv/dash-data');

    const config = getBaseConfigFactory()();

    config.set('core.log.filePath', '/srv/dash-data/logs/core.log');
    config.set('platform.drive.abci.logs.file', {
      destination: '/home/operator/logs/drive.log',
      level: 'info',
      format: 'full',
      color: null,
    });

    const obfuscated = obfuscateConfig(config);

    expect(obfuscated.get('core.log.filePath')).to.equal('/~/logs/core.log');
    expect(obfuscated.get('platform.drive.abci.logs.file.destination')).to.equal('/~/logs/drive.log');
  });

  // An account name can be spelled like a value the config is made of - an
  // environment name, a part of an image name - and masking it there would
  // change what the config says, or make it invalid. It is masked in paths,
  // where it names the operator.
  it('should mask the account name in paths only', function it() {
    this.sinon.stub(os, 'userInfo').returns({ username: 'production', homedir: '/home/production' });

    const config = getBaseConfigFactory()();

    config.set('environment', 'production');
    config.set('core.docker.image', 'dashpay/production:test');
    config.set('core.log.filePath', '/srv/production/core.log');

    const obfuscated = obfuscateConfig(config);

    expect(obfuscated.get('environment')).to.equal('production');
    expect(obfuscated.get('core.docker.image')).to.equal('dashpay/production:test');
    expect(obfuscated.get('core.log.filePath')).to.equal('/srv/**********/core.log');
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
