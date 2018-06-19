xdescribe('Blockchain reorganization', () => {
  before('having started first Dash Drive node, generated STs and second Dash Drive node replicated data from the first one', () => {
    // TODO: start Dash Drive node #1 and #2

    // TODO: generate some STs

    // TODO: wait until Dash Drive #1 saves data

    // TODO: wait until Dash Drive #2 replicate data from Dash Drive #1
  });

  it('Dash Drive should sync data after blockchain reorganization, removing uncessary data.' +
     'Dash Drive on another node should replicate data from the first one.', () => {
    // TODO: get block hash at some height and invalidate it

    // TODO: generate more STs

    // TODO: wait until data is synced

    // TODO: check old data has been removed

    // TODO: wait until data is replicated on the secod node

    // TODO: check data match one of the first node
  });
});
