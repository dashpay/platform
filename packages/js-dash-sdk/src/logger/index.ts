/* eslint-disable @typescript-eslint/no-explicit-any */
const LOG_LEVEL = (typeof process !== 'undefined' && process.env && process.env.LOG_LEVEL) || 'info';

const LEVELS: Record<string, number> = {
  silent: -1, error: 0, warn: 1, info: 2, verbose: 3, debug: 4, silly: 5,
};

export interface ConfigurableLogger {
  error: (...args: any[]) => void;
  warn: (...args: any[]) => void;
  info: (...args: any[]) => void;
  verbose: (...args: any[]) => void;
  debug: (...args: any[]) => void;
  silly: (...args: any[]) => void;
  getForId: (id: string) => ConfigurableLogger;
}

const cache: Record<string, ConfigurableLogger> = {};

function build(level: string = LOG_LEVEL, prefix: string = ''): ConfigurableLogger {
  const threshold = LEVELS[level] != null ? LEVELS[level] : LEVELS.silent;
  const noop = () => { /* no-op */ };
  const fmt = prefix
    ? (...a: any[]) => [prefix, ...a]
    : (...a: any[]) => a;

  const logger: ConfigurableLogger = {
    error: threshold >= 0 ? (...a: any[]) => console.error(...fmt(...a)) : noop,
    warn: threshold >= 1 ? (...a: any[]) => console.warn(...fmt(...a)) : noop,
    info: threshold >= 2 ? (...a: any[]) => console.info(...fmt(...a)) : noop,
    verbose: threshold >= 3 ? (...a: any[]) => console.debug(...fmt(...a)) : noop,
    debug: threshold >= 4 ? (...a: any[]) => console.debug(...fmt(...a)) : noop,
    silly: threshold >= 5 ? (...a: any[]) => console.debug(...fmt(...a)) : noop,
    getForId(id: string): ConfigurableLogger {
      if (!cache[id]) {
        cache[id] = build(level, `[SDK: ${id}]`);
      }
      return cache[id];
    },
  };
  return logger;
}

const logger = build();

logger.verbose(`[SDK] Logger uses "${LOG_LEVEL}" level`, { level: LOG_LEVEL });

export default logger;
