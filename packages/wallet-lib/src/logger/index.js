const util = require('util');
const winston = require('winston');

const LOG_LEVEL = process.env.LOG_LEVEL || 'info';

// Log levels:
//   error    0
//   warn     1
//   info     2  (default)
//   verbose  3
//   debug    4
//   silly    5

const createLogger = (formats = []) => winston.createLogger({
  level: LOG_LEVEL,
  transports: [
    new winston.transports.Console({
      format: winston.format.combine(
        {
          transform: (info) => {
            const args = info[Symbol.for('splat')];
            const result = { ...info };
            if (args) {
              result.message = util.format(info.message, ...args);
            }
            return result;
          },
        },
        ...formats,
        winston.format.colorize(),
        winston.format.printf(({
          level, message,
        }) => `${level}: ${message}`),
      ),
    }),
  ],
});

const logger = createLogger();

const loggers = {};
logger.getForWallet = (walletId) => {
  if (!loggers[walletId]) {
    const format = {
      transform: (info) => {
        const message = `[Wallet: ${walletId}] ${info.message}`;
        return { ...info, message };
      },
    };

    loggers[walletId] = createLogger([format]);
  }

  return loggers[walletId];
};

logger.verbose(`Logger uses "${LOG_LEVEL}" level`, { level: LOG_LEVEL });

module.exports = logger;
