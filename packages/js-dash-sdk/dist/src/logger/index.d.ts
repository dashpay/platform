import { Logger } from 'winston';
export declare type ConfigurableLogger = Logger & {
    getForId: (id: string) => ConfigurableLogger;
};
declare const logger: ConfigurableLogger;
export default logger;
