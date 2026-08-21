import fs from 'fs';
import path from 'path';
import { Listr } from 'listr2';
import HomeDir from '../../../../src/config/HomeDir.js';
import obtainLetsEncryptCertificateTaskFactory from '../../../../src/listr/tasks/ssl/letsencrypt/obtainLetsEncryptCertificateTaskFactory.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import getEnquirerMock from '../../../../src/test/mock/getEnquirerMock.js';
import { ERRORS } from '../../../../src/ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';

describe('obtainLetsEncryptCertificateTaskFactory', () => {
  it('should reject a plaintext ACME directory before lego starts', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      const options = config.getStoredOptions();
      options.externalIp = '127.0.0.1';
      options.platform.gateway.ssl.providerConfigs.letsencrypt.email = 'operator@example.com';
      options.platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl = 'http://acme.example/directory';
      config.setOptions(options, true);

      const missingContainerError = new Error('container not found');
      missingContainerError.statusCode = 404;
      const docker = {
        getContainer: this.sinon.stub().rejects(missingContainerError),
        createContainer: this.sinon.stub().throws(new Error('lego was started')),
      };
      const obtainCertificateTask = obtainLetsEncryptCertificateTaskFactory(
        docker,
        this.sinon.stub().resolves(),
        { addContainer: this.sinon.stub() },
        homeDir,
        this.sinon.stub(),
        this.sinon.stub(),
        null,
        {},
      )(config);

      await expect(obtainCertificateTask.run({ force: true }))
        .to.be.rejectedWith('ACME directory URL must use HTTPS');

      expect(docker.createContainer).to.not.have.been.called();
    } finally {
      homeDir.remove();
    }
  });

  it('should not change config when a valid certificate pair is already installed', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '127.0.0.1');
      config.set('platform.gateway.ssl.enabled', true);
      config.set('platform.gateway.ssl.provider', 'letsencrypt');
      config.set(
        'platform.gateway.ssl.providerConfigs.letsencrypt.email',
        'operator@example.com',
      );
      config.markAsSaved();

      const validateLetsEncryptCertificate = this.sinon.stub().resolves({
        data: {
          certificate: {
            expires: new Date('2026-10-01T00:00:00.000Z'),
          },
          isCertificatePairInstalled: true,
        },
      });
      const saveCertificateTask = this.sinon.stub();
      const obtainCertificateTask = obtainLetsEncryptCertificateTaskFactory(
        {},
        this.sinon.stub(),
        { addContainer: this.sinon.stub() },
        homeDir,
        validateLetsEncryptCertificate,
        saveCertificateTask,
        null,
        {},
      )(config);

      await obtainCertificateTask.run();

      expect(config.isChanged()).to.be.false();
      expect(saveCertificateTask).to.not.have.been.called();
    } finally {
      homeDir.remove();
    }
  });

  it('should select Let’s Encrypt when its valid pair is installed but not configured', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '127.0.0.1');
      config.set(
        'platform.gateway.ssl.providerConfigs.letsencrypt.email',
        'operator@example.com',
      );
      config.markAsSaved();

      const validateLetsEncryptCertificate = this.sinon.stub().resolves({
        data: {
          certificate: {
            expires: new Date('2026-10-01T00:00:00.000Z'),
          },
          isCertificatePairInstalled: true,
        },
      });
      const saveCertificateTask = this.sinon.stub();
      const obtainCertificateTask = obtainLetsEncryptCertificateTaskFactory(
        {},
        this.sinon.stub(),
        { addContainer: this.sinon.stub() },
        homeDir,
        validateLetsEncryptCertificate,
        saveCertificateTask,
        null,
        {},
      )(config);

      await obtainCertificateTask.run();

      expect(config.get('platform.gateway.ssl.enabled')).to.be.true();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('letsencrypt');
      expect(saveCertificateTask).to.not.have.been.called();
    } finally {
      homeDir.remove();
    }
  });

  it('should not select Let’s Encrypt when saving certificate files fails', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '127.0.0.1');
      config.set(
        'platform.gateway.ssl.providerConfigs.letsencrypt.email',
        'operator@example.com',
      );

      const missingContainerError = new Error('container not found');
      missingContainerError.statusCode = 404;
      const docker = {
        getContainer: this.sinon.stub().rejects(missingContainerError),
        createContainer: this.sinon.stub(),
      };
      const externalIp = config.get('externalIp');
      const legoCertificatesDir = homeDir.joinPath(
        config.getName(),
        'platform',
        'gateway',
        'lego',
        'certificates',
      );
      const container = {
        start: this.sinon.stub().resolves(),
        wait: this.sinon.stub().callsFake(async () => {
          fs.writeFileSync(path.join(legoCertificatesDir, `${externalIp}.crt`), 'certificate');
          fs.writeFileSync(path.join(legoCertificatesDir, `${externalIp}.key`), 'private-key');

          return { StatusCode: 0 };
        }),
      };
      docker.createContainer.resolves(container);

      const saveCertificateTask = this.sinon.stub().returns(new Listr([
        {
          task: async () => {
            throw new Error('save failed');
          },
        },
      ]));
      const obtainCertificateTask = obtainLetsEncryptCertificateTaskFactory(
        docker,
        this.sinon.stub().resolves(),
        { addContainer: this.sinon.stub() },
        homeDir,
        this.sinon.stub(),
        saveCertificateTask,
        null,
        {},
      )(config);

      await expect(obtainCertificateTask.run({ force: true }))
        .to.be.rejectedWith('save failed');

      expect(config.get('platform.gateway.ssl.enabled')).to.be.false();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('zerossl');
    } finally {
      homeDir.remove();
    }
  });

  it('should finish installing an already issued certificate after a save failure', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '127.0.0.1');
      config.set(
        'platform.gateway.ssl.providerConfigs.letsencrypt.email',
        'operator@example.com',
      );

      const externalIp = config.get('externalIp');
      const legoCertificatesDir = homeDir.joinPath(
        config.getName(),
        'platform',
        'gateway',
        'lego',
        'certificates',
      );
      fs.mkdirSync(legoCertificatesDir, { recursive: true });
      const legoCertPath = path.join(legoCertificatesDir, `${externalIp}.crt`);
      const legoKeyPath = path.join(legoCertificatesDir, `${externalIp}.key`);
      fs.writeFileSync(legoCertPath, 'certificate');
      fs.writeFileSync(legoKeyPath, 'private-key');

      const validateLetsEncryptCertificate = this.sinon.stub().resolves({
        data: {
          certificate: {
            expires: new Date('2026-10-01T00:00:00.000Z'),
          },
          isBundleFilePresent: true,
          isCertificatePairInstalled: false,
          isPrivateKeyFilePresent: true,
          legoCertPath,
          legoKeyPath,
        },
      });
      let saveAttempts = 0;
      const saveCertificateTask = this.sinon.stub().callsFake(() => new Listr([
        {
          task: async () => {
            saveAttempts += 1;

            if (saveAttempts === 1) {
              throw new Error('save failed');
            }

            config.set('platform.gateway.ssl.enabled', true);
          },
        },
      ]));
      const missingContainerError = new Error('container not found');
      missingContainerError.statusCode = 404;
      const docker = {
        getContainer: this.sinon.stub().rejects(missingContainerError),
        createContainer: this.sinon.stub().resolves({
          start: this.sinon.stub().resolves(),
          wait: this.sinon.stub().resolves({ StatusCode: 0 }),
        }),
      };
      const obtainCertificateTask = obtainLetsEncryptCertificateTaskFactory(
        docker,
        this.sinon.stub(),
        { addContainer: this.sinon.stub() },
        homeDir,
        validateLetsEncryptCertificate,
        saveCertificateTask,
        null,
        {},
      );

      await expect(obtainCertificateTask(config).run({ force: true }))
        .to.be.rejectedWith('save failed');
      await expect(obtainCertificateTask(config).run())
        .to.not.be.rejected();

      expect(saveCertificateTask).to.have.been.calledTwice();
      expect(config.get('platform.gateway.ssl.enabled')).to.be.true();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('letsencrypt');
    } finally {
      homeDir.remove();
    }
  });

  describe('contactless issuance', () => {
    let homeDir;
    let config;
    let legoDir;

    beforeEach(() => {
      homeDir = HomeDir.createTemp();
      config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '1.2.3.4');
      config.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', null);
      legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
    });

    afterEach(() => homeDir.remove());

    /**
     * A lego container that succeeds and leaves behind the files lego would.
     *
     * @param {Object} sinon
     * @param {number} [statusCode]
     * @return {Object}
     */
    function getDockerMock(sinon, statusCode = 0) {
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });

      return {
        getContainer: sinon.stub().rejects(missing),
        createContainer: sinon.stub().resolves({
          start: sinon.stub().resolves(),
          logs: sinon.stub().resolves(Buffer.from('Timeout during connect (likely firewall problem)')),
          wait: sinon.stub().callsFake(async () => {
            if (statusCode === 0) {
              const certificates = path.join(legoDir, 'certificates');
              fs.mkdirSync(certificates, { recursive: true });
              fs.writeFileSync(path.join(certificates, '1.2.3.4.crt'), 'certificate');
              fs.writeFileSync(path.join(certificates, '1.2.3.4.key'), 'key');
            }

            return { StatusCode: statusCode };
          }),
        }),
      };
    }

    /**
     * @param {Object} sinon
     * @param {Object} [options]
     * @return {Object}
     */
    function buildTask(sinon, { docker, validate, save } = {}) {
      return obtainLetsEncryptCertificateTaskFactory(
        docker ?? getDockerMock(sinon),
        sinon.stub().resolves(),
        { addContainer: sinon.stub() },
        homeDir,
        validate ?? sinon.stub().resolves({ error: ERRORS.CERTIFICATE_NOT_FOUND, data: {} }),
        save ?? sinon.stub().callsFake(() => new Listr([{ task: () => {} }])),
        null,
        {},
      );
    }

    // No new node will have an email: nothing prompts for one any more. A
    // throw left anywhere on this path breaks every fresh setup.
    it('should obtain a certificate with no email configured', async function it() {
      const docker = getDockerMock(this.sinon);
      const saveCertificateTask = this.sinon.stub().callsFake(() => new Listr([{ task: () => {} }]));

      await buildTask(this.sinon, { docker, save: saveCertificateTask })(config).run({ force: true });

      expect(saveCertificateTask).to.have.been.calledOnce();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('letsencrypt');
    });

    // lego keys its on-disk ACME account directory by the email string, so an
    // empty --email is a different account from no --email at all. Passing one
    // would silently register a new account.
    it('should pass no --email argument when none is configured', async function it() {
      const docker = getDockerMock(this.sinon);

      await buildTask(this.sinon, { docker })(config).run({ force: true });

      const { Cmd } = docker.createContainer.firstCall.firstArg;
      expect(Cmd).to.not.include('--email');
      expect(Cmd).to.not.include('');
    });

    it('should still pass an email that is configured', async function it() {
      const docker = getDockerMock(this.sinon);
      config.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', 'operator@example.com');

      await buildTask(this.sinon, { docker })(config).run({ force: true });

      const { Cmd } = docker.createContainer.firstCall.firstArg;
      expect(Cmd).to.include('--email');
      expect(Cmd).to.include('operator@example.com');
    });

    // The helper schedules this exact path whenever the pair is not installed,
    // so falling through to the default case makes an affected node throw
    // "Unknown error" hourly, forever, with no route out.
    it('should install a valid certificate that never reached the gateway', async function it() {
      const docker = getDockerMock(this.sinon);
      const saveCertificateTask = this.sinon.stub().callsFake(() => new Listr([{ task: () => {} }]));
      const validate = this.sinon.stub().resolves({
        error: ERRORS.CERTIFICATE_NOT_INSTALLED,
        data: { certificate: { expires: new Date() }, isCertificatePairInstalled: false },
      });

      // The pair lego already issued is on disk; only the gateway's copy of it
      // is missing, which is the whole point of this case.
      const certificates = path.join(legoDir, 'certificates');
      fs.mkdirSync(certificates, { recursive: true });
      fs.writeFileSync(path.join(certificates, '1.2.3.4.crt'), 'certificate');
      fs.writeFileSync(path.join(certificates, '1.2.3.4.key'), 'key');

      await buildTask(this.sinon, { docker, validate, save: saveCertificateTask })(config).run({});

      expect(saveCertificateTask).to.have.been.calledOnce();
      // Nothing is re-issued: the certificate already exists, it was just never
      // copied to where the gateway reads it.
      expect(docker.createContainer).to.not.have.been.called();
    });
  });

  describe('port 80 retry loop', () => {
    let homeDir;
    let config;
    let legoDir;

    beforeEach(() => {
      homeDir = HomeDir.createTemp();
      config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '1.2.3.4');
      legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
    });

    afterEach(() => homeDir.remove());

    /**
     * @param {Object} sinon
     * @return {Object}
     */
    function getFailingDockerMock(sinon) {
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });

      return {
        getContainer: sinon.stub().rejects(missing),
        createContainer: sinon.stub().resolves({
          start: sinon.stub().resolves(),
          logs: sinon.stub().resolves(
            Buffer.from('Timeout during connect (likely firewall problem)'),
          ),
          wait: sinon.stub().resolves({ StatusCode: 1 }),
        }),
      };
    }

    /**
     * @param {Object} sinon
     * @param {Object} docker
     * @return {Function}
     */
    function buildFailingTask(sinon, docker) {
      return obtainLetsEncryptCertificateTaskFactory(
        docker,
        sinon.stub().resolves(),
        { addContainer: sinon.stub() },
        homeDir,
        sinon.stub().resolves({ error: ERRORS.CERTIFICATE_NOT_FOUND, data: {} }),
        sinon.stub(),
        null,
        {},
      );
    }

    /**
     * @param {Object} tasks
     * @param {Object} enquirer
     * @return {Object}
     */
    function inject(tasks, enquirer) {
      // eslint-disable-next-line no-param-reassign
      tasks.options.injectWrapper = { enquirer };

      return tasks;
    }

    // Every attempt spends one of Let's Encrypt's five failed authorizations
    // per hour, and that budget is shared with the helper's renewal of a
    // still-valid certificate. An immediate retry cannot succeed anyway: the
    // operator has not left the terminal to change a firewall rule.
    it('should offer a capped retry that defaults to No', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const enquirer = getEnquirerMock(this.sinon, true, true);

      const tasks = inject(buildFailingTask(this.sinon, docker)(config), enquirer);

      await expect(tasks.run({ force: true, interactive: true })).to.be.rejected();

      expect(docker.createContainer).to.have.been.calledThrice();
      expect(enquirer.prompt).to.have.been.calledTwice();
      expect(enquirer.options[0].initial).to.equal(false);
      expect(enquirer.options[0].message).to.contain('[attempt 2 of 3]');
      expect(enquirer.options[1].message).to.contain('[attempt 3 of 3]');
    });

    it('should stop as soon as the operator declines', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const enquirer = getEnquirerMock(this.sinon, false);

      const tasks = inject(buildFailingTask(this.sinon, docker)(config), enquirer);

      await expect(tasks.run({ force: true, interactive: true })).to.be.rejected();

      expect(docker.createContainer).to.have.been.calledOnce();
    });

    // A long-failing address may be paused rather than rate-limited, and
    // waiting never clears a pause. Telling the operator to come back in N
    // minutes would be wrong for exactly the nodes that have been dark longest.
    it('should give up with guidance that does not promise waiting will help', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const enquirer = getEnquirerMock(this.sinon, false);

      const tasks = inject(buildFailingTask(this.sinon, docker)(config), enquirer);

      const error = await tasks.run({ force: true, interactive: true }).catch((e) => e);

      expect(error.message).to.contain('https://letsencrypt.org/docs/rate-limits/');
      expect(error.message).to.contain('PAUSED');
      expect(error.message).to.contain(`--config ${config.getName()}`);
      expect(error.message).to.contain('renews under the same');
      expect(error.message).to.not.match(/come back in \d/i);
    });

    // The retry is a prompt, and a prompt reached unattended never settles.
    // Gating it on anything less than a positive opt-in leaves the helper's
    // hourly renewal one refactor away from hanging in a container forever.
    it('should never construct a prompt when the session cannot answer', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const enquirer = getEnquirerMock(this.sinon, true);

      const tasks = inject(buildFailingTask(this.sinon, docker)(config), enquirer);

      await expect(tasks.run({ force: true })).to.be.rejected();

      expect(enquirer.prompt).to.not.have.been.called();
      expect(docker.createContainer).to.have.been.calledOnce();
    });

    it('should honour no-retry even for an operator at a terminal', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const enquirer = getEnquirerMock(this.sinon, true);

      const tasks = inject(buildFailingTask(this.sinon, docker)(config), enquirer);

      await expect(tasks.run({ force: true, interactive: true, noRetry: true })).to.be.rejected();

      expect(enquirer.prompt).to.not.have.been.called();
      expect(docker.createContainer).to.have.been.calledOnce();
    });

    // The gate runs the obtain without --force, so a node whose certificate is
    // still valid never reaches lego however often update is run - which is
    // what keeps repeated runs off the five-certificates-per-week budget.
    it('should not reach lego while the certificate is still valid', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const validate = this.sinon.stub().resolves({
        data: { certificate: { expires: new Date() }, isCertificatePairInstalled: true },
      });

      const tasks = obtainLetsEncryptCertificateTaskFactory(
        docker,
        this.sinon.stub().resolves(),
        { addContainer: this.sinon.stub() },
        homeDir,
        validate,
        this.sinon.stub(),
        null,
        {},
      )(config);

      await tasks.run({});

      expect(docker.createContainer).to.not.have.been.called();
    });
  });
});
