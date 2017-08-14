const { cl, is } = require('khal')

function handleBody(req, res) {
	if (req && req.hasOwnProperty('body')) {
		return req.body;
	}
	return returnResponse({ error: "Missing body data" }, res);
}

function returnResponse(response, res) {
	return res.send(response);
}

function handleRequiredField(body, expectedFields, res, next) {
	let valid = true;
	if (expectedFields && expectedFields.constructor.name === "Object") {
		for (let i = 0; i < Object.keys(expectedFields).length; i++) {
			let param = Object.keys(expectedFields)[i];
			let rules = expectedFields[param];

			if (!body.hasOwnProperty(param) && rules.required !== false) {
				returnResponse({ error: `Missing param ${param}` }, res);
				return false;
			}
			valid = (handleType(rules.type, body, param, rules.value, res));

			if (!valid) {
				return false;
			}//When one of above is false, return false.
		}
	}
	function handleType(type, body, param, rulesValue, res) {
		let paramData = body[param];
		let curryReturn = (data) => { returnResponse(data, res) }
		switch (type) {
			case "enum":
				if (rulesValue.indexOf(paramData) === -1) {
					curryReturn({ error: `Param ${param} has invalid value '${paramData}', expected one of '${rulesValue}'` });
					return false;
				}
				break;
			case "number":
				if (paramData.constructor.name !== "Number" || !Number.isInteger(paramData)) {
					curryReturn({ error: `Param ${param} has invalid type ${paramData.constructor.name} - expected Number` });
					return false;
				}
				break;
			case "json":
				if (paramData.constructor.name !== "Object" || !is.JSON(paramData)) {
					curryReturn({ error: `Param ${param} has invalid type ${paramData.constructor.name} - expected JSON` });
					return false;
				}
				if (JSON.stringify(paramData).length <= 2) {
					curryReturn({ error: `Expected param ${param} to have at least a value - Received empty json` });
					return false;
				}
				break;
			default:
				throw new Error('Not handled type ' + type);
				break;

		}
		return true;
	}
	return true;
}

class Handlers {
	constructor(app) {
		let debug = app.logger.debug;
		let quorum = app.quorum;
		let insight = app.insight;
		let authService = app.authService;

		return {
			post: {
				quorum: function(req, res, next) {
					let body = handleBody(req, res);
					if (!handleRequiredField(body, {
						verb: { required: true, type: 'enum', value: ['add', 'commit', 'remove', 'state', 'listen', 'migrate', 'auth'] },
						qid: { required: true, type: 'number' },
						data: { required: true, type: 'json' }
					}, res)) {
						//If field doesn't meet required rules, will be returned false and enter here in order to break
						//continuation of the logic
						return next();
					}
					//At this point, we know we have required field with expected type.
					switch (body.verb) {
						case "add":
							returnResponse(quorum.performAction('add', { qid: body.qid, data: body.data }), res);
							break;
						case "commit":
							returnResponse(quorum.performAction('commit', { qid: body.qid, data: body.data }), res);
							break;
						case "remove":
							returnResponse(quorum.performAction('remove', { qid: body.qid, data: body.data }), res);
							break;
						default:
							returnResponse(`Not Implemented`, res);
							break;
					}
				},
				tx: {
					send: function(req, res) {
						let rawTX = req.body.rawtx;
						insight.performPOSTRequest('/tx', { rawtx: rawTX }, req, res);
					},
					sendix: function(req, res) {
						let rawTX = req.body.rawtx;
						insight.performPOSTRequest('/tx/sendix', { rawtx: rawTX }, req, res);
					}
				}
			},
			get: {
				blocks: function(req, res) {
					insight.performGETRequest('/blocks', req, res)
				},
				blockHeight: function(req, res) {
					let height = req.params.height;
					insight.performGETRequest('/block-index/' + height, req, res)
				},
				blockHash: function(req, res) {
					let hash = req.params.hash;
					insight.performGETRequest('/block/' + hash, req, res)
				},
				rawBlock: function(req, res) {
					let blockHash = req.body.blockHash;
					insight.performGETRequest('/rawblock/' + blockHash, req, res)
				},
				tx: {
					get: function(req, res) {
						let txID = req.params.txid;
						insight.performGETRequest('/tx/' + txID, req, res)
					}
				},
				currency: function(req, res) {
					insight.performGETRequest('/currency', req, res)
				},
				status: function(req, res) {

					if (req.query.q) {
						insight.performGETRequest(`/status?q=${req.query.q}`, req, res);
					}
					else {
						insight.performGETRequest('/status', req, res);
					}
				},
				sync: function(req, res) {
					insight.performGETRequest('/sync', req, res)
				},
				peer: function(req, res) {
					insight.performGETRequest('/peer', req, res)
				},
				version: function(req, res) {
					insight.performGETRequest('/version', req, res)
				},
				address: {
					get: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/addr/' + addr, req, res)
					},
					utxo: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/addr/' + addr + '/utxo', req, res)
					},
					utxos: function(req, res) {
						let addrs = req.params.addrs;
						insight.performGETRequest('/addrs/' + addr + '/utxo', req, res)
					},
					txs: function(req, res) {
						let addrs = req.params.addrs;
						insight.performGETRequest('/addrs/' + addr + '/txs', req, res)
					},
					balance: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/addr/' + addr + '/balance', req, res)
					},
					totalReceived: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/addr/' + addr + '/totalReceived', req, res)
					},
					totalSent: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/addr/' + addr + '/totalSent', req, res)
					},
					unconfirmedBalance: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/addr/' + addr + '/unconfirmedBalance', req, res)
					},
				},
				utils: {
					estimatefee: function(req, res) {
						let addr = req.params.addr;
						insight.performGETRequest('/utils/estimatefee', req, res)
						// does not exist on insight servers?
					}
				},
				hello: function(req, res) {
					res.send('Hello World!');
				},
				info: function(req, res) {
					//This could be used in order to return app.rpc.getInfo();
					res.send('Unavailable');
				},
				mnList: function(req, res) {
					//todo: get from insight-api
					res.send([
						{
							"vin": "f31bd0fcb34317a3db0dbf607c899d022c9e8a9d712c94c87bac953caafcf1a2-1",
							"status": "ENABLED",
							"rank": 1,
							"ip": "127.0.0.1:3000",
							"protocol": 70206,
							"payee": "Xjd6yGfWcsuDcHdECCwn3XUScLb3q3ChJK",
							"activeseconds": 14556663,
							"lastseen": 1502078628
						},
						{
							"vin": "f31bd0fcb34317a3db0dbf607c899d022c9e8a9d712c94c87bac953caafcf1a2-1",
							"status": "ENABLED",
							"rank": 1,
							"ip": "52.202.12.210:9999",
							"protocol": 70206,
							"payee": "Xjd6yGfWcsuDcHdECCwn3XUScLb3q3ChJK",
							"activeseconds": 14556663,
							"lastseen": 1502078628
						}, {
							"vin": "c097539c6d03e45d8933b92d7cd702c6906b74f16860ddaedb33107e940fea27-0",
							"status": "ENABLED",
							"rank": 2,
							"ip": "37.59.247.180:9999",
							"protocol": 70206,
							"payee": "Xk8LPywDnggrAQaVWTkLk1JMabs1ZRLWT5",
							"activeseconds": 312936,
							"lastseen": 1502078652
						}, {
							"vin": "f1de44e05cfc54ef70b6f2769f3ef9289c858ea7f0356012836ec9d0581f5ea3-1",
							"status": "ENABLED",
							"rank": 3,
							"ip": "45.76.178.221:9999",
							"protocol": 70206,
							"payee": "XpwGvnRwjbMmT1hyA6nS5NesdJk4d4bE3y",
							"activeseconds": 3714536,
							"lastseen": 1502078813
						}, {
							"vin": "2326359e1e0fed73063f0330d0d5d32d70ddfb427e7135fc3d678be49ac9022b-1",
							"status": "ENABLED",
							"rank": 4,
							"ip": "46.101.153.25:9999",
							"protocol": 70206,
							"payee": "XyxESWr2mPz5JQmrX5w1TQBRLMgM5MoE1K",
							"activeseconds": 12092763,
							"lastseen": 1502078485
						}, {
							"vin": "1297deb0f9cbd1443114ac15cb2fe42a69a182de9ebdf2b109114f61d2283798-1",
							"status": "ENABLED",
							"rank": 5,
							"ip": "178.62.218.126:9999",
							"protocol": 70206,
							"payee": "Xog5c8MG6Qneu1hZfLuUwjWqaWjD6Fbrdk",
							"activeseconds": 11796620,
							"lastseen": 1502078972
						}])

				},
				auth: {
					getChallenge: (req, res) => {
						res.send(authService.getChallenge(req.params.identifier));
					},
					//add further routes when specs are defined
				}
			}
		}
	}
}
module.exports = Handlers;