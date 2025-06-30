import { Logger, Logform } from 'winston';

export type ConfigurableLogger = Logger & {
  getForId: (id: string) => ConfigurableLogger
};

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

const createLogger = (formats: Logform.Format[] = []): ConfigurableLogger => winston.createLogger({
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
logger.getForId = (id: string): ConfigurableLogger => {
  if (!loggers[id]) {
    const format = {
      transform: (info) => {
        const message = `[SDK: ${id}] ${info.message}`;
        return { ...info, message };
      },
    };

    loggers[id] = createLogger([format]);
  }

  return loggers[id];
};

logger.verbose(`[SDK] Logger uses "${LOG_LEVEL}" level`, { level: LOG_LEVEL });

export default logger;
