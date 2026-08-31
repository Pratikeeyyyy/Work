const { expect } = require("chai");
const { ethers } = require("hardhat");

const oneEth = ethers.parseEther("1");

describe("FreelanceEscrow", function () {
  async function deployEscrow(opts = {}) {
    const [client, freelancer, mediator, stranger] = await ethers.getSigners();
    const value = opts.value ?? oneEth;
    const freelancerAddr = opts.freelancer ?? freelancer.address;
    const mediatorAddr = opts.mediator ?? (opts.noMediator ? ethers.ZeroAddress : mediator.address);

    const Factory = await ethers.getContractFactory("FreelanceEscrow");
    const escrow = await Factory.connect(client).deploy(freelancerAddr, mediatorAddr, { value });
    return { escrow, client, freelancer, mediator, stranger, value };
  }

  it("deploys funded in state Funded with correct metadata", async function () {
    const { escrow, client, freelancer, mediator, value } = await deployEscrow();
    expect(await escrow.client()).to.equal(client.address);
    expect(await escrow.freelancer()).to.equal(freelancer.address);
    expect(await escrow.mediator()).to.equal(mediator.address);
    expect(await escrow.amount()).to.equal(value);
    expect(await escrow.state()).to.equal(0);
    const info = await escrow.getInfo();
    expect(info.state).to.equal(0);
    expect(info.amount).to.equal(value);
  });

  it("succeeds end to end: startWork -> submitWork -> approve pays freelancer", async function () {
    const { escrow, client, freelancer, value } = await deployEscrow();

    await escrow.connect(freelancer).startWork();
    expect(await escrow.state()).to.equal(1);

    await escrow.connect(freelancer).submitWork();
    expect(await escrow.state()).to.equal(2);

    await expect(() => escrow.connect(client).approve()).to.changeEtherBalances(
      [freelancer.address, await escrow.getAddress()],
      [value, -value]
    );
    expect(await escrow.state()).to.equal(3);
  });

  it("rejects deploys with no deposit or bad freelancer", async function () {
    const [client, freelancer] = await ethers.getSigners();
    const Factory = await ethers.getContractFactory("FreelanceEscrow");

    await expect(
      Factory.connect(client).deploy(freelancer.address, ethers.ZeroAddress, { value: 0 })
    ).to.be.revertedWithCustomError(Factory, "ZeroAmount");

    await expect(
      Factory.connect(client).deploy(ethers.ZeroAddress, ethers.ZeroAddress, { value: oneEth })
    ).to.be.revertedWithCustomError(Factory, "ZeroAddress");
  });

  it("lets the client cancel before the freelancer starts and reclaims funds", async function () {
    const { escrow, client, value } = await deployEscrow();
    await expect(() => escrow.connect(client).cancelBeforeWork()).to.changeEtherBalances(
      [client.address, await escrow.getAddress()],
      [value, -value]
    );
    expect(await escrow.state()).to.equal(5);
  });

  it("resolves a dispute via the mediator", async function () {
    const { escrow, client, freelancer, mediator, value } = await deployEscrow();

    await escrow.connect(freelancer).startWork();
    await escrow.connect(freelancer).submitWork();
    await escrow.connect(client).raiseDispute();
    expect(await escrow.state()).to.equal(4);

    const freelancerShare = value / 2n;
    await expect(() => escrow.connect(mediator).resolveDispute(freelancerShare)).to.changeEtherBalances(
      [freelancer.address, client.address],
      [freelancerShare, value - freelancerShare]
    );
    expect(await escrow.state()).to.equal(3);
  });

  it("locks the dispute settlement when no mediator is configured", async function () {
    const { escrow, client, freelancer, mediator, stranger } = await deployEscrow({ noMediator: true });

    await escrow.connect(freelancer).startWork();
    await escrow.connect(client).raiseDispute();
    expect(await escrow.state()).to.equal(4);

    await expect(escrow.connect(mediator).resolveDispute(1)).to.be.revertedWithCustomError(
      escrow,
      "OnlyMediator"
    );
    await expect(escrow.connect(stranger).resolveDispute(1)).to.be.revertedWithCustomError(
      escrow,
      "OnlyMediator"
    );
  });

  it("enforces the deadline for client refund", async function () {
    const { escrow, client, value } = await deployEscrow();
    const deadline = await escrow.deadline();

    await expect(escrow.connect(client).refundAfterDeadline()).to.be.revertedWithCustomError(
      escrow,
      "DeadlineNotPassed"
    );

    await ethers.provider.send("evm_setNextBlockTimestamp", [Number(deadline) + 1]);
    await expect(() => escrow.connect(client).refundAfterDeadline()).to.changeEtherBalances(
      [client.address, await escrow.getAddress()],
      [value, -value]
    );
    expect(await escrow.state()).to.equal(5);
  });

  it("locks non-parties out of lifecycle functions", async function () {
    const { escrow, stranger } = await deployEscrow();

    await expect(escrow.connect(stranger).startWork()).to.be.revertedWithCustomError(
      escrow,
      "OnlyFreelancer"
    );
    await expect(escrow.connect(stranger).approve()).to.be.revertedWithCustomError(
      escrow,
      "OnlyClient"
    );
  });

  it("rejects direct funding after deployment", async function () {
    const { escrow, freelancer, value } = await deployEscrow();
    await expect(
      freelancer.sendTransaction({ to: await escrow.getAddress(), value })
    ).to.be.revertedWith("no direct funding");
  });
});