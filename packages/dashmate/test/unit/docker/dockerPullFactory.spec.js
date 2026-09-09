import { PassThrough } from 'node:stream';
import Docker from 'dockerode';
import dockerPullFactory from '../../../src/docker/dockerPullFactory.js';

describe('dockerPull', () => {
  let stream;
  let docker;
  let dockerPull;

  beforeEach(() => {
    stream = new PassThrough();

    // The real modem is used on purpose: it owns the stream buffering and
    // decides what counts as a failed pull
    docker = {
      modem: new Docker().modem,
      pull: (image, callback) => callback(null, stream),
    };

    dockerPull = dockerPullFactory(docker);
  });

  it('should reject when Docker Hub rate limits the pull', async () => {
    const message = 'toomanyrequests: You have reached your pull rate limit';

    const promise = dockerPull('dashpay/drive:4');

    stream.write(`${JSON.stringify({ errorDetail: { message }, error: message })}\n`);
    stream.end();

    await expect(promise).to.be.rejectedWith(message);
  });

  it('should reject when the host runs out of disk space during the pull', async () => {
    const message = 'write /var/lib/docker/tmp: no space left on device';

    const promise = dockerPull('dashpay/drive:4');

    stream.write(`${JSON.stringify({ status: 'Downloading', id: 'a1b2c3' })}\n`);
    stream.write(`${JSON.stringify({ errorDetail: { message }, error: message })}\n`);
    stream.end();

    await expect(promise).to.be.rejectedWith(message);
  });

  it('should reject when the daemon refuses the pull', async () => {
    docker.pull = (image, callback) => callback(new Error('connect ENOENT /var/run/docker.sock'));

    await expect(dockerPull('dashpay/drive:4'))
      .to.be.rejectedWith('connect ENOENT /var/run/docker.sock');
  });

  it('should reject when the pull stream fails', async () => {
    const promise = dockerPull('dashpay/drive:4');

    stream.destroy(new Error('socket hang up'));

    await expect(promise).to.be.rejectedWith('socket hang up');
  });

  it('should resolve when the pull succeeds', async () => {
    const promise = dockerPull('dashpay/drive:4');

    stream.write(`${JSON.stringify({ status: 'Status: Downloaded newer image for dashpay/drive:4' })}\n`);
    stream.end();

    const output = await promise;

    expect(output).to.have.lengthOf(1);
    expect(output[0].status).to.equal('Status: Downloaded newer image for dashpay/drive:4');
  });
});
