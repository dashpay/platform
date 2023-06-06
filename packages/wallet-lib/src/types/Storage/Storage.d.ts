export declare class Storage {
    constructor(options?: Storage.Options);
}

export declare namespace Storage {
    interface Options {
        rehydrate: boolean,
        autosave: boolean,
        purgeOnError: boolean,
        autosaveIntervalTime: number,
        network: string
    }
}