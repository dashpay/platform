const explorerGet = require('./common/ExplorerHelper').explorerGet;

exports.getMasternodeList = function() {
	return new Promise(function(resolve, reject) {
		explorerGet(`/masternodes/list`)
		.then(data => {
			resolve(data);
		})
		.catch(error => {
			reject(`An error was triggered while fetching masternode list :` + error);
	})
	});
}
