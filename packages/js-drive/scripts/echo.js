const net = require('net');

async function sendEcho(ip) {
  const echoRequestBytes = Buffer.from('0a0a080a0668656c6c6f21', 'hex');

  return new Promise((resolve, reject) => {
    const client = net.connect(26658, ip);

    client.on('connect', () => {
      client.write(echoRequestBytes);
    });

    client.on('data', () => {
      client.destroy();

      resolve('ok');
    });

    client.on('error', reject);

    setTimeout(() => {
      reject(new Error('Can\'t connect to ABCI port: timeout.'));
    }, 2000);
  });
}

sendEcho('127.0.0.1')
  // eslint-disable-next-line no-console
  .then(console.log)
  .catch((e) => {
    // eslint-disable-next-line no-console
    console.error(e);
    process.exit(1);
  });
