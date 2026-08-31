// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title FreelanceEscrow
/// @notice A trust-minimized escrow for freelance gigs found via LeadGen.
///
///  Flow:
///    1. The client deploys the contract, naming the freelancer and an optional
///       mediator, and deposits the agreed amount (msg.value) in the same tx.
///    2. The freelancer marks the work as in progress, then submits the deliverable.
///    3. The client approves and the freelancer is paid. On disagreement either
///       party can raise a dispute and the mediator splits the funds.
///    4. If no one acts before the deadline the client can withdraw the escrow.
///
///  Direct EOAs only: no ERC20, no reentrancy risk beyond a guarded send.
contract FreelanceEscrow {
    enum State {
        Funded,      // 0 - deposited, awaiting the freelancer
        InProgress,  // 1 - freelancer started work
        Submitted,   // 2 - deliverable handed over, awaiting approval
        Completed,   // 3 - funds paid to the freelancer
        Disputed,    // 4 - either party raised a dispute
        Refunded     // 5 - client withdrew the escrow
    }

    struct Info {
        address client;
        address freelancer;
        address mediator;
        uint256 amount;
        uint256 deadline;
        State state;
        uint256 submittedAt;
    }

    address public immutable client;
    address public immutable freelancer;
    /// @notice Optional third party that resolves disputes. Zero address disables disputes.
    address public immutable mediator;
    uint256 public immutable amount;
    uint256 public immutable deadline;

    State public state;
    uint256 public submittedAt;

    event Funded(address indexed client, address indexed freelancer, uint256 amount);
    event WorkStarted();
    event WorkSubmitted(uint256 at);
    event Approved(address indexed client);
    event DisputeRaised(address indexed by);
    event DisputeResolved(address indexed freelancer, uint256 share);
    event Refunded(address indexed client, uint256 amount);

    error OnlyClient();
    error OnlyFreelancer();
    error OnlyMediator();
    error NotParty();
    error WrongState(State expected);
    error ZeroAddress();
    error ZeroAmount();
    error BadShare();
    error DeadlineNotPassed();
    error NothingToRefund();
    error TransferFailed();

    modifier onlyClient() {
        if (msg.sender != client) revert OnlyClient();
        _;
    }

    modifier onlyFreelancer() {
        if (msg.sender != freelancer) revert OnlyFreelancer();
        _;
    }

    modifier onlyMediator() {
        if (msg.sender != mediator) revert OnlyMediator();
        _;
    }

    modifier inState(State expected) {
        if (state != expected) revert WrongState(expected);
        _;
    }

    /// @param _freelancer Address that will deliver the work
    /// @param _mediator Optional dispute resolver; use address(0) to opt out
    constructor(address _freelancer, address _mediator) payable {
        if (_freelancer == address(0)) revert ZeroAddress();
        if (msg.value == 0) revert ZeroAmount();

        client = msg.sender;
        freelancer = _freelancer;
        mediator = _mediator;
        amount = msg.value;
        deadline = block.timestamp + 30 days;
        state = State.Funded;

        emit Funded(client, freelancer, amount);
    }

    /// @notice Freelancer confirms they have started the work.
    function startWork() external onlyFreelancer inState(State.Funded) {
        state = State.InProgress;
        emit WorkStarted();
    }

    /// @notice Freelancer hands over the deliverable; approval window follows.
    function submitWork() external onlyFreelancer inState(State.InProgress) {
        state = State.Submitted;
        submittedAt = block.timestamp;
        emit WorkSubmitted(submittedAt);
    }

    /// @notice Client accepts the deliverable and pays the freelancer in full.
    function approve() external onlyClient inState(State.Submitted) {
        state = State.Completed;
        _send(freelancer, amount);
        emit Approved(client);
    }

    /// @notice Client cancels before the freelancer submits anything.
    function cancelBeforeWork() external onlyClient inState(State.Funded) {
        state = State.Refunded;
        _send(client, amount);
        emit Refunded(client, amount);
    }

    /// @notice After the deadline, a client whose escrow is not complete or stuck
    ///         in a dispute may pull the funds back.
    function refundAfterDeadline() external onlyClient {
        if (block.timestamp < deadline) revert DeadlineNotPassed();
        if (state == State.Completed || state == State.Disputed || state == State.Refunded) {
            revert NothingToRefund();
        }
        state = State.Refunded;
        _send(client, amount);
        emit Refunded(client, amount);
    }

    /// @notice Either party can escalate a disagreement to the mediator.
    function raiseDispute() external {
        if (msg.sender != client && msg.sender != freelancer) revert NotParty();
        if (state != State.InProgress && state != State.Submitted) revert WrongState(State.InProgress);
        state = State.Disputed;
        emit DisputeRaised(msg.sender);
    }

    /// @notice The mediator settles the dispute. `freelancerShareWei` goes to the
    ///         freelancer, the remainder comes back to the client.
    function resolveDispute(uint256 freelancerShareWei) external onlyMediator inState(State.Disputed) {
        if (freelancerShareWei > amount) revert BadShare();
        state = State.Completed;
        _send(freelancer, freelancerShareWei);
        uint256 rest = amount - freelancerShareWei;
        if (rest > 0) _send(client, rest);
        emit DisputeResolved(freelancer, freelancerShareWei);
    }

    /// @notice One-call view for dashboards.
    function getInfo() external view returns (Info memory) {
        return Info(client, freelancer, mediator, amount, deadline, state, submittedAt);
    }

    receive() external payable {
        revert("no direct funding");
    }

    function _send(address to, uint256 value) private {
        (bool ok, ) = payable(to).call{value: value}("");
        if (!ok) revert TransferFailed();
    }
}