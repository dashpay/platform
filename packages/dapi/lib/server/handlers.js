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
		let insight = app.insight;
		let authService = app.authService;
		let quorum = app.quorum;
		let qTempPort = app.config.server.port; //QDEVTEMP - remove

		return {
			post: {
				quorum: function(req, res, next) {
					let body = handleBody(req, res);

					if (!handleRequiredField(body, {
						verb: { required: true, type: 'enum', value: ['add', 'commit', 'remove', 'state', 'listen', 'migrate', 'auth'] },
						qid: { required: true, type: 'number' },
						data: { required: true, type: 'json' },
					}, res)) {
						//If field doesn't meet required rules, will be returned false and enter here in order to break
						//continuation of the logic
						return next();
					}

					quorum.isValidQuorum(body, qTempPort)
						.then(isValid => {
							if (!isValid) {
								returnResponse(quorum.getQuroumFailedResponse(), res);
							}
							else {
								returnResponse(quorum.performAction(body.verb, { qid: body.qid, data: body.data }), res);
							}
						})
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
					insight.getMnList()
						.then(l => {
							res.send(l)
						})
				},
				mnUpdateList: function(req, res) {
					insight.getMnUpdateList(req.params.hash)
						.then(l => {
							res.send(l)
						})
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