"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.extend = void 0;
function extend(Derived, Base) {
    Object.setPrototypeOf(Derived.prototype, Base.prototype);
}
exports.extend = extend;
