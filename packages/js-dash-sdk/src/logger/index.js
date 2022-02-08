const pino = require('pino');
const config = require('config');


require('dotenv').config();

const ENABLE_DEBUG_LOGS = (process.env.ENABLE_DEBUG_LOGS === "true");

const logger = pino({
    level: 'debug',
    enabled: ENABLE_DEBUG_LOGS,
    crlf: false,
    timestamp: true,
    messageKey: 'msg',
    transport: {
        target: 'pino-pretty',
    },
});


module.exports = logger;
