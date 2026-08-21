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

    beforeEach(() => {
      homeDir = HomeDir.createTemp();
      config = getBaseConfigFactory(homeDir)();
      config.set('externalIp', '1.2.3.4');
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

    // When another process holds port 80, Docker refuses the port binding and
    // lego never starts: no ACME request is made, so nothing is rate-limited,
    // nothing can be paused, and the firewall is not the problem - the port is
    // reachable, it is occupied. Branching on the request never having been
    // attempted, rather than on what the authority said, is what keeps this
    // out of the business of classifying provider output.
    it('should not blame the firewall or the authority when the helper never started', async function it() {
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });
      const bindRefused = Object.assign(
        new Error('(HTTP code 500) server error - failed to set up container networking:'
          + ' failed to bind host port 0.0.0.0:80/tcp: address already in use'),
        { statusCode: 500 },
      );
      const docker = {
        getContainer: this.sinon.stub().rejects(missing),
        createContainer: this.sinon.stub().resolves({
          start: this.sinon.stub().rejects(bindRefused),
          logs: this.sinon.stub().resolves(Buffer.from('')),
          wait: this.sinon.stub().resolves({ StatusCode: 0 }),
        }),
      };

      const error = await buildFailingTask(this.sinon, docker)(config)
        .run({ force: true }).catch((e) => e);

      // The Docker error is shown, because it is the only thing that says what
      // actually happened, and the port conflict is offered as a possible
      // cause rather than asserted as the cause.
      expect(error.message).to.contain('address already in use');
      expect(error.message).to.contain('holding port 80');
      expect(error.message).to.contain('no request was');

      // And none of the authority-side consequences are claimed.
      expect(error.message).to.not.contain('PAUSED');
      expect(error.message).to.not.contain('rate-limit');
      expect(error.message).to.not.contain('failed attempts are shared');
      expect(error.message).to.not.contain('Fix inbound port 80 first');
    });

    // lego exited successfully, so a certificate was issued and counts against
    // this node's weekly limit whether or not dashmate can find the files.
    // Retrying that as though the authority had refused spends the limit again,
    // up to three times, and then reports that nothing was obtained.
    it('should not retry or blame the authority when the issued files are missing', async function it() {
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });
      const docker = {
        getContainer: this.sinon.stub().rejects(missing),
        createContainer: this.sinon.stub().resolves({
          start: this.sinon.stub().resolves(),
          logs: this.sinon.stub().resolves(Buffer.from('')),
          // Exits cleanly, but writes nothing.
          wait: this.sinon.stub().resolves({ StatusCode: 0 }),
        }),
      };

      const context = { force: true, interactive: true };
      const error = await buildFailingTask(this.sinon, docker)(config)
        .run(context).catch((e) => e);

      // Issued once, and only once.
      expect(docker.createContainer).to.have.been.calledOnce();

      // The issuance is the fact the operator needs: it happened, and it cost
      // one of the five this address gets each week.
      expect(error.message).to.match(/issued a certificate/i);
      expect(error.message).to.match(/issuance limit/i);

      // Rerunning obtains a replacement, because files that were never written
      // cannot be recovered. Saying it reinstalls what already exists would
      // contradict the sentence above it about the limit.
      expect(error.message).to.not.match(/install what was already issued/i);

      // Printed once, by the command, not also here.
      expect(error.message).to.not.contain('LEAVE PORT 80 OPEN');
      expect(error.message).to.not.contain('PAUSED');
      expect(error.message).to.not.contain('failed attempts are shared');
      expect(error.message).to.not.match(/did not obtain a certificate after/i);

      // And the operator still hears the requirement that keeps the node up.
      expect(context.certificateObtained).to.be.true();
    });

    // Clearing a stale container from a previous run happens before lego is
    // even created, so a failure there is as far from a certificate authority
    // response as a bind refusal is.
    it('should not blame the authority when clearing a stale container fails', async function it() {
      const denied = Object.assign(new Error('permission denied while removing container'), { statusCode: 403 });
      const docker = {
        getContainer: this.sinon.stub().resolves({
          remove: this.sinon.stub().rejects(denied),
          wait: this.sinon.stub().resolves(),
        }),
        createContainer: this.sinon.stub(),
      };

      const error = await buildFailingTask(this.sinon, docker)(config)
        .run({ force: true }).catch((e) => e);

      expect(error.message).to.contain('permission denied while removing container');
      expect(error.message).to.not.contain('PAUSED');
      expect(error.message).to.not.contain('failed attempts are shared');
      expect(docker.createContainer).to.not.have.been.called();
    });

    // The Docker error is shown because only it says what happened; asserting
    // the port is occupied for a daemon, permission or configuration failure
    // would send the operator after the wrong thing.
    it('should not assert a port conflict it did not observe', async function it() {
      const daemonGone = Object.assign(new Error('Cannot connect to the Docker daemon'), { statusCode: 500 });
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });
      const docker = {
        getContainer: this.sinon.stub().rejects(missing),
        createContainer: this.sinon.stub().rejects(daemonGone),
      };

      const error = await buildFailingTask(this.sinon, docker)(config)
        .run({ force: true }).catch((e) => e);

      expect(error.message).to.contain('Cannot connect to the Docker daemon');
      expect(error.message).to.not.match(/the port is occupied/i);
      expect(error.message).to.not.match(/is already listening on port 80/i);
      // Nor any of the guidance that belongs to a response from the authority.
      expect(error.message).to.not.contain('PAUSED');
      expect(error.message).to.not.contain('failed attempts are shared');
      expect(error.message).to.not.contain('Fix inbound port 80 first');
      // It may say no limit was spent - that is the honest statement. What it
      // must not do is discuss a limit as though one had been.
      expect(error.message).to.contain('no request was');
      expect(error.message).to.not.match(/may be PAUSED|Self-Service Portal/i);
    });

    // The container ran, so a request may well have been made - but dashmate
    // never saw the result, so it cannot report what the authority said either.
    it('should say the result was never read when the wait fails', async function it() {
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });
      const docker = {
        getContainer: this.sinon.stub().rejects(missing),
        createContainer: this.sinon.stub().resolves({
          start: this.sinon.stub().resolves(),
          logs: this.sinon.stub().resolves(Buffer.from('')),
          wait: this.sinon.stub().rejects(new Error('connection reset by peer')),
        }),
      };

      const error = await buildFailingTask(this.sinon, docker)(config)
        .run({ force: true }).catch((e) => e);

      expect(error.message).to.contain('connection reset by peer');
      expect(error.message).to.match(/could not read|did not see/i);
      expect(error.message).to.not.contain('PAUSED');
    });

    // A failure the authority did return keeps the guidance that is about the
    // authority.
    it('should keep the rate-limit guidance when the request did reach the authority', async function it() {
      const docker = getFailingDockerMock(this.sinon);

      const error = await buildFailingTask(this.sinon, docker)(config)
        .run({ force: true }).catch((e) => e);

      expect(error.message).to.contain('PAUSED');
      expect(error.message).to.contain('failed attempts are shared');
    });

    // lego fails for reasons that have nothing to do with the firewall - a rate
    // limit, an account problem, a bad directory - and naming port 80 as the
    // cause of all of them sends an operator to check something that is fine.
    // Its own output says what happened; the prompt should not overrule it.
    it('should not blame port 80 for every failure', async function it() {
      const docker = getFailingDockerMock(this.sinon);
      const enquirer = getEnquirerMock(this.sinon, false);

      const tasks = inject(buildFailingTask(this.sinon, docker)(config), enquirer);

      await expect(tasks.run({ force: true, interactive: true })).to.be.rejected();

      const { header } = enquirer.options[0];

      expect(header).to.contain('Timeout during connect');
      expect(header).to.not.match(/could not reach [0-9.]+ on port 80/i);
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

      const error = await tasks.run({ force: true }).catch((e) => e);

      expect(enquirer.prompt).to.not.have.been.called();
      expect(docker.createContainer).to.have.been.calledOnce();

      // lego's own account of the failure is what the operator needs. Reaching
      // the prompt and being refused there would be safe but would replace it
      // with a report that dashmate tried to ask a question, so the decision
      // not to retry has to be made before the prompt is reached.
      expect(error.message).to.contain('Timeout during connect');
      expect(error.message).to.not.contain('without a terminal');
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
