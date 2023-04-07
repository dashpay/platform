"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (_) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
var chai_1 = require("chai");
var resolveByRecord_1 = __importDefault(require("./resolveByRecord"));
describe('Platform', function () {
    describe('Names', function () {
        describe('#resolveByRecord', function () {
            var platformMock;
            var parentDomainDocument;
            beforeEach(function beforeEach() {
                return __awaiter(this, void 0, void 0, function () {
                    return __generator(this, function (_a) {
                        parentDomainDocument = { label: 'parent' };
                        platformMock = {
                            documents: {
                                get: this.sinon.stub(),
                            },
                            initialize: this.sinon.stub(),
                        };
                        return [2 /*return*/];
                    });
                });
            });
            it('should resolve domain by it\'s record', function () { return __awaiter(void 0, void 0, void 0, function () {
                var receivedDocuments;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0:
                            platformMock.documents.get.resolves([parentDomainDocument]);
                            return [4 /*yield*/, resolveByRecord_1.default.call(platformMock, 'recordName', 'recordValue')];
                        case 1:
                            receivedDocuments = _a.sent();
                            chai_1.expect(platformMock.documents.get.callCount).to.equal(1);
                            chai_1.expect(platformMock.documents.get.getCall(0).args).to.deep.equal([
                                'dpns.domain',
                                {
                                    where: [['records.recordName', '==', 'recordValue']],
                                },
                            ]);
                            chai_1.expect(receivedDocuments).to.deep.equal([parentDomainDocument]);
                            return [2 /*return*/];
                    }
                });
            }); });
            it('should return null if domain was not found', function () { return __awaiter(void 0, void 0, void 0, function () {
                var receivedDocuments;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0:
                            platformMock.documents.get.resolves([]);
                            return [4 /*yield*/, resolveByRecord_1.default.call(platformMock, 'recordName', 'recordValue')];
                        case 1:
                            receivedDocuments = _a.sent();
                            chai_1.expect(platformMock.documents.get.callCount).to.equal(1);
                            chai_1.expect(platformMock.documents.get.getCall(0).args).to.deep.equal([
                                'dpns.domain',
                                {
                                    where: [['records.recordName', '==', 'recordValue']],
                                },
                            ]);
                            chai_1.expect(receivedDocuments).to.be.an('array');
                            chai_1.expect(receivedDocuments.length).to.equal(0);
                            return [2 /*return*/];
                    }
                });
            }); });
        });
    });
});
//# sourceMappingURL=resolveByRecord.spec.js.map