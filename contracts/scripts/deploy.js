const { ethers } = require("hardhat");

async function main() {
  const [deployer] = await ethers.getSigners();

  const freelancer = process.env.FREELANCER_ADDRESS || deployer.address;
  const mediator = process.env.MEDIATOR_ADDRESS || ethers.ZeroAddress;
  const amount = ethers.parseEther(process.env.AMOUNT_ETH || "0.1");

  const FreelanceEscrow = await ethers.getContractFactory("FreelanceEscrow");
  const escrow = await FreelanceEscrow.deploy(freelancer, mediator, { value: amount });
  await escrow.waitForDeployment();

  const address = await escrow.getAddress();
  console.log("FreelanceEscrow deployed to:", address);
  console.log("  client (deployer):", deployer.address);
  console.log("  freelancer:      ", freelancer);
  console.log("  mediator:        ", mediator);
  console.log("  deposit (ETH):   ", ethers.formatEther(amount));
}

main().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});