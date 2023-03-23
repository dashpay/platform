function validatePort(port) {
  const portNumber = Math.floor(Number(port));

  return portNumber >= 1
    && portNumber <= 65535
    && portNumber.toString() === port;
}

module.exports = validatePort;
