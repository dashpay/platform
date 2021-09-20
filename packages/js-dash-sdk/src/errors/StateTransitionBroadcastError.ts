export class StateTransitionBroadcastError extends Error {
    code: number;
    message: string;
    cause: Error;

    /**
     * @param {number} code
     * @param {string} message
     * @param {Error} cause
     */
    constructor(code: number, message: string, cause: Error) {
        super(message);

        this.code = code;
        this.message = message;
        this.cause = cause;

        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, this.constructor);
        }

        Object.setPrototypeOf(this, StateTransitionBroadcastError.prototype);
    }

    /**
     * Returns error code
     *
     * @return {number}
     */
    getCode(): number {
        return this.code;
    }

    /**
     * Returns error message
     *
     * @return {string}
     */
    getMessage(): string {
        return this.message;
    }

    /**
     * Get error that was a cause
     *
     * @return {Error}
     */
    getCause(): any {
        return this.cause;
    }
}
