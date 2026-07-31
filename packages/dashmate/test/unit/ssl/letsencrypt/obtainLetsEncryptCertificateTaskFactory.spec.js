import fs from 'fs';
import path from 'path';
import { Listr } from 'listr2';
import HomeDir from '../../../../src/config/HomeDir.js';
import obtainLetsEncryptCertificateTaskFactory from '../../../../src/listr/tasks/ssl/letsencrypt/obtainLetsEncryptCertificateTaskFactory.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';

describe('obtainLetsEncryptCertificateTaskFactory', () => {
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
});
