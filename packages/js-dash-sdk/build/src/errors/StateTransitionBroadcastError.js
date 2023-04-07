"use strict";
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.StateTransitionBroadcastError = void 0;
var StateTransitionBroadcastError = /** @class */ (function (_super) {
    __extends(StateTransitionBroadcastError, _super);
    /**
       * @param {number} code
       * @param {string} message
       * @param {Error} cause
       */
    function StateTransitionBroadcastError(code, message, cause) {
        var _this = _super.call(this, message) || this;
        _this.code = code;
        _this.message = message;
        _this.cause = cause;
        if (Error.captureStackTrace) {
            Error.captureStackTrace(_this, _this.constructor);
        }
        Object.setPrototypeOf(_this, StateTransitionBroadcastError.prototype);
        return _this;
    }
    /**
       * Returns error code
       *
       * @return {number}
       */
    StateTransitionBroadcastError.prototype.getCode = function () {
        return this.code;
    };
    /**
       * Returns error message
       *
       * @return {string}
       */
    StateTransitionBroadcastError.prototype.getMessage = function () {
        return this.message;
    };
    /**
       * Get error that was a cause
       *
       * @return {Error}
       */
    StateTransitionBroadcastError.prototype.getCause = function () {
        return this.cause;
    };
    return StateTransitionBroadcastError;
}(Error));
exports.StateTransitionBroadcastError = StateTransitionBroadcastError;
//# sourceMappingURL=StateTransitionBroadcastError.js.map