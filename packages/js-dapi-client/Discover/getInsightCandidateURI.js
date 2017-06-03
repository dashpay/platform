exports.getInsightCandidateURI = function() {

    return new Promise(function(resolve, reject) {
        SDK.Discover.getInsightCandidate()
            .then(candidate => {
                resolve(candidate.URI)
            })
            .catch(err => {
                reject(err);
            })
    });
}