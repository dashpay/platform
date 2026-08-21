import fs from 'fs';
import path from 'path';
import { Listr } from 'listr2';
import HomeDir from '../../src/config/HomeDir.js';
import Config from '../../src/config/Config.js';
import getBaseConfigFactory from '../../configs/defaults/getBaseConfigFactory.js';
import analyseGatewayCertificateFactory from '../../src/doctor/analyse/analyseGatewayCertificateFactory.js';
import Samples from '../../src/doctor/Samples.js';
import gatewayCertificateTaskFactory from '../../src/listr/tasks/update/gatewayCertificateTaskFactory.js';
import obtainLetsEncryptCertificateTaskFactory from '../../src/listr/tasks/ssl/letsencrypt/obtainLetsEncryptCertificateTaskFactory.js';
import saveCertificateTaskFactory from '../../src/listr/tasks/ssl/saveCertificateTask.js';
import renderCertificateGuidance from '../../src/ssl/renderCertificateGuidance.js';
import { CERTIFICATE_REASONS, CERTIFICATE_STATUS } from '../../src/ssl/checkGatewayCertificateFactory.js';
import { issueCertificate } from '../../src/test/certificateFixtures.js';
import getEnquirerMock from '../../src/test/mock/getEnquirerMock.js';

// Deliberately not the default. A command missing its config still works
// against the default node, so a test driven from the default config passes
// whether or not the config is there at all.
const CONFIG_NAME = 'testnet_2';
const EXTERNAL_IP = '1.2.3.4';

/**
 * The subcommands dashmate actually has. Anchoring on these separates a command
 * from prose that merely names dashmate ("dashmate did not open a connection"),
 * without having to guess at sentence shape.
 */
const SUBCOMMANDS = [
  'config', 'core', 'doctor', 'group', 'logs', 'reset', 'restart', 'setup',
  'ssl', 'start', 'status', 'stop', 'update', 'wallet',
];

const COMMAND = new RegExp(`dashmate\\s+(?:${SUBCOMMANDS.join('|')})\\b[^\\n}\`'"]*`, 'g');

/**
 * A backticked mention of nothing but the command's own name, as in "this does
 * not block `dashmate start`". Naming a command is not telling someone to run
 * it, and there is nothing for a node name to attach to.
 */
const BARE_REFERENCE = new RegExp(`\`dashmate\\s+(?:${SUBCOMMANDS.join('|')})\``, 'g');

/**
 * Presentation forms used in source: a chalk-highlighted command, or an
 * indented line in a message template.
 */
const PRESENTED_IN_SOURCE = new RegExp(
  `(?:\\{bold\\.cyanBright\\s+|^[ \\t]{4,})(dashmate\\s+(?:${SUBCOMMANDS.join('|')})\\b[^\\n}\`'"]*)`,
  'gm',
);

/**
 * Files that render an operator-copyable command and predate this work, where
 * every command is bare by existing convention. They are listed rather than
 * skipped by pattern so a NEW file cannot join them silently - which is the
 * whole point of the sweep below.
 */
const PRE_EXISTING_BARE_COMMANDS = [
  'src/commands/doctor/index.js',
  'src/commands/setup.js',
  'src/doctor/analyse/analyseConfigFactory.js',
  'src/doctor/analyse/analyseCoreFactory.js',
  'src/doctor/analyse/analyseServiceContainersFactory.js',
  'src/listr/tasks/setup/regular/getConfigurationOutputFromContext.js',
  'src/listr/tasks/setup/setupRegularPresetTaskFactory.js',
];

/**
 * @param {string} directory
 * @param {string[]} [found]
 * @return {string[]}
 */
function javascriptFilesIn(directory, found = []) {
  fs.readdirSync(directory, { withFileTypes: true }).forEach((entry) => {
    const entryPath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      javascriptFilesIn(entryPath, found);
    } else if (entry.name.endsWith('.js')) {
      found.push(entryPath);
    }
  });

  return found;
}

/**
 * Every command in rendered output that an operator could copy and run.
 *
 * @param {string} text
 * @return {string[]}
 */
function commandsIn(text) {
  const references = new Set(
    [...text.matchAll(BARE_REFERENCE)].map((match) => match[0].replace(/`/g, '')),
  );

  return [...text.matchAll(COMMAND)]
    .map((match) => match[0].trim().replace(/[.,;:]+$/, ''))
    .filter((command) => !references.has(command));
}

/**
 * @param {string} source
 * @return {string[]}
 */
function presentedCommandsIn(source) {
  return [...source.matchAll(PRESENTED_IN_SOURCE)].map((match) => match[1].trim());
}

describe('every command dashmate tells an operator to run', () => {
  let homeDir;
  let config;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    config = new Config(CONFIG_NAME, getBaseConfigFactory(homeDir)().getOptions());
    config.set('network', 'mainnet');
    config.set('externalIp', EXTERNAL_IP);
  });

  afterEach(() => homeDir.remove());

  /**
   * @param {Object} [overrides]
   * @return {Object}
   */
  const verdict = (overrides = {}) => ({
    status: CERTIFICATE_STATUS.INVALID,
    reasons: [{ code: CERTIFICATE_REASONS.EXPIRED, message: 'the certificate expired' }],
    warnings: [],
    skipped: [],
    provider: config.get('platform.gateway.ssl.provider'),
    installed: { validTo: new Date(Date.now() + 6 * 864e5) },
    expiresInDays: 6,
    ...overrides,
  });

  /**
   * @param {string} label
   * @param {string} text
   */
  function expectEveryCommandNamesTheNode(label, text) {
    const commands = commandsIn(text);

    expect(commands, `${label} rendered no command to check`).to.have.length.greaterThan(0);

    commands.forEach((command) => {
      expect(command, `${label}: ${command}`).to.contain(`--config ${CONFIG_NAME}`);
    });
  }

  // The rendered surfaces, driven for real rather than read from source, so a
  // command assembled at runtime is checked too.
  describe('as rendered', () => {
    ['zerossl', 'letsencrypt', 'file', 'self-signed'].forEach((provider) => {
      it(`names the node in the guidance for a ${provider} node`, () => {
        config.set('platform.gateway.ssl.provider', provider);

        [
          renderCertificateGuidance({
            config, verdict: verdict(), isNodeRunning: false, pull: null,
          }),
          renderCertificateGuidance({
            config, verdict: verdict(), isNodeRunning: true, pull: { ok: true, failed: 0, total: 3 },
          }),
          renderCertificateGuidance({
            config,
            verdict: verdict({
              reasons: [{
                code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
                message: 'a switch was interrupted',
              }],
            }),
            isNodeRunning: false,
            pull: null,
          }),
        ].forEach((output, index) => expectEveryCommandNamesTheNode(`guidance ${provider}/${index}`, output));
      });
    });

    it('names the node in every doctor prescription', () => {
      const samples = new Samples();
      samples.setDashmateConfig(config);
      samples.setServiceInfo('gateway', 'installedCertificate', {
        status: 'INVALID',
        reasons: [{ code: 'EXPIRED', message: 'expired' }],
        warnings: [{ code: 'EXPIRING_SOON', message: 'expires tomorrow' }],
      });
      samples.setServiceInfo('gateway', 'servedCertificate', {
        state: 'served',
        port: 443,
        certificate: { fingerprint256: 'AA:BB', validTo: new Date(Date.now() - 864e5).toUTCString() },
        chainVerified: false,
        chainError: 'untrusted',
        identityVerified: true,
        matchesOnDisk: false,
      });

      const problems = analyseGatewayCertificateFactory()(samples);

      expect(problems).to.have.length.greaterThan(2);
      problems.forEach((problem, index) => expectEveryCommandNamesTheNode(
        `doctor problem ${index}`,
        problem.getSolution(),
      ));
    });

    it('names the node when the operator gives up on port 80', async function it() {
      const missing = Object.assign(new Error('no container'), { statusCode: 404 });
      const tasks = obtainLetsEncryptCertificateTaskFactory(
        {
          getContainer: this.sinon.stub().rejects(missing),
          createContainer: this.sinon.stub().resolves({
            start: this.sinon.stub().resolves(),
            logs: this.sinon.stub().resolves(Buffer.from('Timeout during connect')),
            wait: this.sinon.stub().resolves({ StatusCode: 1 }),
          }),
        },
        this.sinon.stub().resolves(),
        { addContainer: this.sinon.stub() },
        homeDir,
        this.sinon.stub().resolves({ error: 'CERTIFICATE_NOT_FOUND', data: {} }),
        this.sinon.stub(),
        null,
        {},
      )(config);

      const error = await tasks.run({ force: true }).catch((e) => e);

      expectEveryCommandNamesTheNode('give-up guidance', error.message);
    });

    it('names the node when a written pair does not match', async () => {
      const certificate = issueCertificate({ ip: EXTERNAL_IP });
      const other = issueCertificate({ ip: EXTERNAL_IP });

      const error = await saveCertificateTaskFactory(homeDir)(config).run({
        certificateFile: certificate.pem,
        privateKeyFile: other.keyPem,
      }).catch((e) => e);

      expectEveryCommandNamesTheNode('save verification', error.message);
    });

    it('names the node in every prompt the check can raise', async function it() {
      const enquirer = getEnquirerMock(this.sinon, false, false);

      const gatewayCertificateTask = gatewayCertificateTaskFactory(
        () => verdict(),
        this.sinon.stub().callsFake(() => new Listr([{ task: () => {} }], { renderer: 'silent' })),
        this.sinon.stub().callsFake(() => new Listr([{ task: () => {} }], { renderer: 'silent' })),
        { isExclusive: () => true, write: this.sinon.stub() },
        {},
        this.sinon.stub(),
        { execCommand: this.sinon.stub().resolves() },
      );

      const tasks = new Listr(
        [{ task: gatewayCertificateTask(config, { interactive: true }) }],
        { renderer: 'silent', exitOnError: false },
      );
      tasks.options.injectWrapper = { enquirer };

      await tasks.run({});

      expect(enquirer.options).to.have.length.greaterThan(0);
      enquirer.options.forEach((option, index) => {
        commandsIn(option.header ?? '').forEach((command) => {
          expect(command, `prompt ${index}: ${command}`).to.contain(`--config ${CONFIG_NAME}`);
        });
      });
    });
  });

  // `dashmate ssl obtain` installs the pair and signals the gateway, and the
  // signal reaches Envoy's hot-restarter, which re-execs Envoy against the same
  // configuration without touching the container. Telling an operator to
  // restart after obtaining therefore buys them an outage and changes nothing.
  //
  // Asserted over every rendered remedy at once rather than per site, because a
  // surface nobody thought to check is exactly where this reappears.
  describe('remedies routed through ssl obtain', () => {
    /**
     * @param {string} label
     * @param {string} text
     * @return {boolean} whether this text prescribed an obtain
     */
    function expectNoRestartAlongsideObtain(label, text) {
      const commands = commandsIn(text);
      const obtains = commands.filter((command) => /dashmate\s+ssl\s+obtain/.test(command));
      const restarts = commands.filter((command) => /dashmate\s+restart/.test(command));

      if (obtains.length === 0) {
        return false;
      }

      expect(restarts, `${label} prescribes an obtain and then ${restarts.join(', ')}`)
        .to.be.empty();

      return true;
    }

    it('never tells the operator to restart afterwards in the update guidance', () => {
      let covered = 0;

      ['zerossl', 'letsencrypt', 'file', 'self-signed'].forEach((provider) => {
        config.set('platform.gateway.ssl.provider', provider);

        [true, false].forEach((isNodeRunning) => {
          const text = renderCertificateGuidance({
            config, verdict: verdict(), isNodeRunning, pull: null,
          });

          if (expectNoRestartAlongsideObtain(`guidance ${provider}/${isNodeRunning}`, text)) {
            covered += 1;
          }
        });
      });

      expect(covered, 'no guidance variant prescribed an obtain').to.equal(8);
    });

    it('never tells the operator to restart afterwards in a doctor prescription', () => {
      /**
       * @param {Object} servedCertificate
       * @return {Object[]}
       */
      const problemsFor = (servedCertificate) => {
        const samples = new Samples();
        samples.setDashmateConfig(config);
        samples.setServiceInfo('gateway', 'installedCertificate', {
          status: 'INVALID',
          reasons: [{ code: 'EXPIRED', message: 'expired' }],
          warnings: [],
        });
        samples.setServiceInfo('gateway', 'servedCertificate', servedCertificate);

        return analyseGatewayCertificateFactory()(samples);
      };

      const expired = new Date(Date.now() - 864e5).toUTCString();
      const base = {
        state: 'served',
        port: 443,
        chainVerified: true,
        identityVerified: true,
        matchesOnDisk: true,
      };

      // Every branch that can prescribe an obtain: the address it served does
      // not belong to this node, its certificate has run out, and its
      // certificate differs from a disk copy not known to be usable.
      const cases = [
        { ...base, identityVerified: false, identityError: 'not in the cert altnames' },
        { ...base, certificate: { fingerprint256: 'AA:BB', validTo: expired } },
        {
          ...base,
          certificate: { fingerprint256: 'AA:BB', validTo: expired },
          matchesOnDisk: false,
          onDisk: { fingerprint256: 'CC:DD' },
        },
        {
          ...base,
          certificate: { fingerprint256: 'AA:BB', validTo: new Date(Date.now() + 864e5).toUTCString() },
          matchesOnDisk: false,
          onDisk: { fingerprint256: 'CC:DD' },
        },
      ];

      let covered = 0;

      cases.forEach((servedCertificate, index) => {
        problemsFor(servedCertificate).forEach((problem, position) => {
          if (expectNoRestartAlongsideObtain(`doctor ${index}/${position}`, problem.getSolution())) {
            covered += 1;
          }
        });
      });

      expect(covered, 'no doctor prescription offered an obtain').to.be.greaterThan(2);
    });
  });

  // The backstop. Rendering can only check surfaces a test knows about, and
  // this class of defect has recurred by arriving in a place nobody thought to
  // check. Anything under src/ that lays out a command has to name the node,
  // and a file joining the exemption list is a visible edit rather than a
  // silent one.
  describe('as written', () => {
    it('lays out no command anywhere in src that cannot name the node', () => {
      const offenders = [];

      javascriptFilesIn('src').forEach((file) => {
        if (PRE_EXISTING_BARE_COMMANDS.includes(file)) {
          return;
        }

        presentedCommandsIn(fs.readFileSync(file, 'utf8')).forEach((command) => {
          if (!/--config|\$\{cfg|renderConfigFlag/.test(command)) {
            offenders.push(`${file}: ${command}`);
          }
        });
      });

      expect(offenders, offenders.join('\n')).to.be.empty();
    });

    it('keeps the exemption list honest', () => {
      PRE_EXISTING_BARE_COMMANDS.forEach((file) => {
        expect(fs.existsSync(file), `${file} is listed but gone`).to.be.true();
        expect(
          presentedCommandsIn(fs.readFileSync(file, 'utf8')),
          `${file} no longer needs an exemption`,
        ).to.have.length.greaterThan(0);
      });
    });
  });
});
