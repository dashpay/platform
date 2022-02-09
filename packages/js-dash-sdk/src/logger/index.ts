import { pino } from 'pino';


const ENABLE_DEBUG_LOGS = (process.env.ENABLE_DEBUG_LOGS === "true");

export const logger = pino({
    level: 'debug',
    enabled: ENABLE_DEBUG_LOGS,
    crlf: false,
    timestamp: true,
    messageKey: 'msg',
    transport: {
        target: 'pino-pretty',
    },
});

