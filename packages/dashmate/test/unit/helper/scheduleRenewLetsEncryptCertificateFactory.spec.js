import fs from 'fs';
import path from 'path';
import LegoCertificate from '../../../src/ssl/letsencrypt/LegoCertificate.js';
import scheduleRenewLetsEncryptCertificateFactory from '../../../src/helper/scheduleRenewLetsEncryptCertificateFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import ConfigIsNotPresentError from '../../../src/config/errors/ConfigIsNotPresentError.js';
import RenewalRecordRepository from '../../../src/ssl/renewalRecord/RenewalRecordRepository.js';

describe('scheduleRenewLetsEncryptCertificateFactory', () => {
  let config;
  let obtainLetsEncryptCertificateTask;
  let dockerCompose;
  let configFileRepository;
  let writeConfigTemplates;
  let scheduleRenewLetsEncryptCertificate;

  beforeEach(function beforeEach() {
    config = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('base'),
    };
    config.get.withArgs('platform.gateway.ssl.enabled').returns(true);
    config.get.withArgs('platform.gateway.ssl.provider').returns('letsencrypt');
    config.get.withArgs('externalIp').returns('127.0.0.1');
    config.isChanged = this.sinon.stub().returns(false);

    obtainLetsEncryptCertificateTask = this.sinon.stub();
    dockerCompose = { execCommand: this.sinon.stub() };
    configFileRepository = {
      read: this.sinon.stub().returns({
        getConfig: this.sinon.stub().returns(config),
      }),
      acquire: this.sinon.stub(),
      isExclusive: () => true,
      readAndMigrate: this.sinon.stub().returns({
        configFile: {
          getConfig: this.sinon.stub().returns(config),
        },
      }),
      write: this.sinon.stub(),
      release: this.sinon.stub(),
    };
    writeConfigTemplates = this.sinon.stub();

    scheduleRenewLetsEncryptCertificate = scheduleRenewLetsEncryptCertificateFactory(
      obtainLetsEncryptCertificateTask,
      dockerCompose,
      configFileRepository,
      writeConfigTemplates,
      {
        joinPath: this.sinon.stub().returns('/tmp/lego'),
      },
      new RenewalRecordRepository(HomeDir.createTemp()),
    );
  });

  it('should run renewal through the fresh locked configuration path', async function it() {
    const clock = this.sinon.useFakeTimers({
      now: new Date('2026-07-31T00:00:00.000Z'),
    });
    const run = this.sinon.stub().resolves();
    obtainLetsEncryptCertificateTask.returns({ run });
    config.isChanged.returns(true);
    this.sinon.stub(LegoCertificate, 'fromFile').returns({
      expires: new Date('2026-08-01T00:00:00.000Z'),
      isExpiredInDays: this.sinon.stub().returns(true),
    });

    await scheduleRenewLetsEncryptCertificate(config);
    await clock.tickAsync(3001);

    expect(configFileRepository.acquire).to.have.been.calledOnce();
    expect(configFileRepository.readAndMigrate).to.have.been.calledOnce();
    expect(obtainLetsEncryptCertificateTask).to.have.been.calledOnceWith(config);
    expect(run).to.have.been.calledOnceWith({
      expirationDays: LegoCertificate.EXPIRATION_LIMIT_DAYS,
      noRetry: true,
    });
    expect(configFileRepository.write).to.have.been.calledOnce();
    expect(writeConfigTemplates).to.have.been.calledOnceWith(config);
    expect(configFileRepository.release).to.have.been.calledOnce();
    expect(dockerCompose.execCommand)
      .to.have.been.calledOnceWith(config, 'gateway', 'kill -SIGHUP 1');
  });

  it('should hand off when the config was removed before scheduling completed', async function it() {
    const onConfigurationChanged = this.sinon.stub().resolves();
    configFileRepository.read.returns({
      getConfig: this.sinon.stub().throws(new ConfigIsNotPresentError('base')),
    });

    await scheduleRenewLetsEncryptCertificate(config, onConfigurationChanged);

    expect(onConfigurationChanged).to.have.been.calledOnceWith(null);
    expect(obtainLetsEncryptCertificateTask).to.not.have.been.called();
  });

  it('should retry an incomplete gateway installation instead of waiting for expiry', async function it() {
    const homeDir = HomeDir.createTemp();
    const clock = this.sinon.useFakeTimers({
      now: new Date('2026-07-31T00:00:00.000Z'),
    });

    try {
      const legoDir = homeDir.joinPath('base', 'platform', 'gateway', 'lego');
      const legoCertificatesDir = path.join(legoDir, 'certificates');
      const gatewayDir = homeDir.joinPath('base', 'platform', 'gateway', 'ssl');
      fs.mkdirSync(legoCertificatesDir, { recursive: true });
      fs.mkdirSync(gatewayDir, { recursive: true });
      fs.writeFileSync(path.join(legoCertificatesDir, '127.0.0.1.crt'), 'new-certificate');
      fs.writeFileSync(path.join(legoCertificatesDir, '127.0.0.1.key'), 'new-key');
      fs.writeFileSync(path.join(gatewayDir, 'bundle.crt'), 'old-certificate');
      fs.writeFileSync(path.join(gatewayDir, 'private.key'), 'old-key');

      const certificate = {
        expires: new Date('2026-10-01T00:00:00.000Z'),
        isExpiredInDays: this.sinon.stub(),
      };
      certificate.isExpiredInDays.onFirstCall().returns(true);
      certificate.isExpiredInDays.onSecondCall().returns(false);
      this.sinon.stub(LegoCertificate, 'fromFile').returns(certificate);

      const run = this.sinon.stub();
      run.onFirstCall().rejects(new Error('save failed'));
      run.onSecondCall().resolves();
      obtainLetsEncryptCertificateTask.returns({ run });

      scheduleRenewLetsEncryptCertificate = scheduleRenewLetsEncryptCertificateFactory(
        obtainLetsEncryptCertificateTask,
        dockerCompose,
        configFileRepository,
        writeConfigTemplates,
        homeDir,
        new RenewalRecordRepository(homeDir),
      );

      await scheduleRenewLetsEncryptCertificate(config);
      await clock.tickAsync(3001);
      await clock.tickAsync((60 * 60 * 1000) + 3001);

      expect(run).to.have.been.calledTwice();
    } finally {
      homeDir.remove();
    }
  });
});
