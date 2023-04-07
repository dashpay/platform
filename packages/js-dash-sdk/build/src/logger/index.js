"use strict";
var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
var __spreadArrays = (this && this.__spreadArrays) || function () {
    for (var s = 0, i = 0, il = arguments.length; i < il; i++) s += arguments[i].length;
    for (var r = Array(s), k = 0, i = 0; i < il; i++)
        for (var a = arguments[i], j = 0, jl = a.length; j < jl; j++, k++)
            r[k] = a[j];
    return r;
};
Object.defineProperty(exports, "__esModule", { value: true });
var util = require('util');
var winston = require('winston');
var LOG_LEVEL = process.env.LOG_LEVEL || 'info';
// Log levels:
//   error    0
//   warn     1
//   info     2  (default)
//   verbose  3
//   debug    4
//   silly    5
var createLogger = function (formats) {
    var _a;
    if (formats === void 0) { formats = []; }
    return winston.createLogger({
        level: LOG_LEVEL,
        transports: [
            new winston.transports.Console({
                format: (_a = winston.format).combine.apply(_a, __spreadArrays([{
                        transform: function (info) {
                            var args = info[Symbol.for('splat')];
                            var result = __assign({}, info);
                            if (args) {
                                result.message = util.format.apply(util, __spreadArrays([info.message], args));
                            }
                            return result;
                        },
                    }], formats, [winston.format.colorize(), winston.format.printf(function (_a) {
                        var level = _a.level, message = _a.message;
                        return level + ": " + message;
                    })])),
            }),
        ],
    });
};
var logger = createLogger();
var loggers = {};
logger.getForId = function (id) {
    if (!loggers[id]) {
        var format = {
            transform: function (info) {
                var message = "[SDK: " + id + "] " + info.message;
                return __assign(__assign({}, info), { message: message });
            },
        };
        loggers[id] = createLogger([format]);
    }
    return loggers[id];
};
logger.verbose("[SDK] Logger uses \"" + LOG_LEVEL + "\" level", { level: LOG_LEVEL });
exports.default = logger;
//# sourceMappingURL=index.js.map