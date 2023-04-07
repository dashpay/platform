export declare class StateTransitionBroadcastError extends Error {
    code: number;
    message: string;
    cause: Error;
    /**
       * @param {number} code
       * @param {string} message
       * @param {Error} cause
       */
    constructor(code: number, message: string, cause: Error);
    /**
       * Returns error code
       *
       * @return {number}
       */
    getCode(): number;
    /**
       * Returns error message
       *
       * @return {string}
       */
    getMessage(): string;
    /**
       * Get error that was a cause
       *
       * @return {Error}
       */
    getCause(): any;
}
