import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import saveCertificateTaskFactory from '../../../src/listr/tasks/ssl/saveCertificateTask.js';

describe('saveCertificateTaskFactory', () => {
  it('should restore the previous certificate pair when saving the key fails', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const config = getBaseConfigFactory(homeDir)();
      config.set('platform.gateway.ssl.enabled', true);
      const certificatesDir = homeDir.joinPath(
        config.getName(),
        'platform',
        'gateway',
        'ssl',
      );
      fs.mkdirSync(certificatesDir, { recursive: true });
      const certificatePath = path.join(certificatesDir, 'bundle.crt');
      const keyPath = path.join(certificatesDir, 'private.key');
      fs.writeFileSync(certificatePath, 'old-certificate');
      fs.writeFileSync(keyPath, 'old-key');

      const originalRenameSync = fs.renameSync.bind(fs);
      this.sinon.stub(fs, 'renameSync').callsFake((source, destination) => {
        if (destination === keyPath) {
          throw new Error('key replace failed');
        }

        return originalRenameSync(source, destination);
      });

      const task = saveCertificateTaskFactory(homeDir)(config);

      await expect(task.run({
        certificateFile: 'new-certificate',
        privateKeyFile: 'new-key',
      })).to.be.rejectedWith('key replace failed');

      expect(fs.readFileSync(certificatePath, 'utf8')).to.equal('old-certificate');
      expect(fs.readFileSync(keyPath, 'utf8')).to.equal('old-key');
      expect(fs.readdirSync(certificatesDir).filter((name) => name.includes('.tmp-')))
        .to.be.empty();
      expect(config.get('platform.gateway.ssl.enabled')).to.be.true();
    } finally {
      homeDir.remove();
    }
  });
});
