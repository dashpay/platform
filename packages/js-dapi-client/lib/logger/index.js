const util = require('util');
const winston = require('winston');

const LOG_LEVEL = process.env.LOG_LEVEL || 'info';
const LOG_TO_FILE = process.env.LOG_WALLET_TO_FILE || 'false';

// Log levels:
//   error    0
//   warn     1
//   info     2  (default)
//   verbose  3
//   debug    4
//   silly    5

const loggers = {};

const createLogger = (formats = [], id = '') => {
  const format = winston.format.combine(
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
  );

  const transports = [
    new winston.transports.Console({
      format,
    }),
  ];

  if (LOG_TO_FILE === 'true' && typeof window === 'undefined') {
    transports.push(
      new winston.transports.File({
        filename: `wallet${id !== '' ? `_${id}` : ''}`,
        format,
      }),
    );
  }

  return winston.createLogger({
    level: LOG_LEVEL,
    transports,
  });
};

const logger = createLogger();

logger.getForId = (id) => {
  if (!loggers[id]) {
    const format = {
      transform: (info) => {
        const message = `[DAPIClient: ${id}] ${info.message}`;
        return { ...info, message };
      },
    };

    loggers[id] = createLogger([format], id);
  }

  return loggers[id];
};

logger.verbose(`Logger uses "${LOG_LEVEL}" level`, { level: LOG_LEVEL });

module.exports = logger;
