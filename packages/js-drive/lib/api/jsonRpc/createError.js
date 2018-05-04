const { server } = require('jayson');

const createError = server.prototype.error.bind(server);

module.exports = createError;
