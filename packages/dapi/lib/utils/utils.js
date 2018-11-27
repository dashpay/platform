const net = require('net');

const isPortTaken = (port, ipv6 = false) => new Promise(((resolve, reject) => {
  try {
    const uri = ipv6 ? '::' : '127.0.0.1';
    const servlet = net.createServer();
    servlet.on('error', (error) => {
      if (error.code !== 'EADDRINUSE') {
        reject(error);
      }
      resolve(true);
    });
    servlet.on('listening', () => {
      servlet.close();
      resolve(false);
    });
    servlet.on('close', () => resolve(false));
    servlet.listen(port, uri);
  } catch (error) {
    reject(error);
  }
}));

module.exports = {
  isPortTaken,
};
