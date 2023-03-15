import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { SystemProgram, Connection, clusterApiUrl, Keypair, PublicKey, sendAndConfirmTransaction, Transaction, Secp256k1Program } from "@solana/web3.js";
import { BN, min } from "bn.js";
import { expect } from "chai";
import { GachaMarketplace } from "../target/types/gacha_marketplace";
import { createKeypairFromFile } from './utils';
import { createMint, createAssociatedTokenAccount, getAccount, getMint, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID, getAssociatedTokenAddress, createAssociatedTokenAccountInstruction, ASSOCIATED_TOKEN_PROGRAM_ID, getMinimumBalanceForRentExemptAccount } from "@solana/spl-token";
import * as borsh from "borsh";

describe("gacha_nft_marketplace", () => {
	const provider = anchor.AnchorProvider.local(
		// "http://localhost:8899"
		// "https://solana-devnet.g.alchemy.com/v2/wRDcdu07s9RATBEH4sdhTA8P6FQNeWJh"
		// "https://api.devnet.solana.com/"
	)
	anchor.setProvider(provider);

	const program = anchor.workspace.GachaMarketplace as Program<GachaMarketplace>;
	let owner;
	let tokenAccount1;
	let tokenAccount2;
	let tokenAccount3;
	let minterNft1;
	let minterNft2;
	let minterNft3;
	let buyer;
	it("Mint", async () => {
		owner = await createKeypairFromFile(__dirname + "/../../../../tungleanh/.config/solana/id.json");
		minterNft1 = await createMint(provider.connection, owner, owner.publicKey, null, 0)
		minterNft2 = await createMint(provider.connection, owner, owner.publicKey, null, 0)
		minterNft3 = await createMint(provider.connection, owner, owner.publicKey, null, 0)
		
		buyer = await createKeypairFromFile(__dirname + "/../../../my-solana-wallet/my-keypair.json");
		await provider.connection.requestAirdrop(buyer.publicKey, 1e9);

		tokenAccount1 = await createAssociatedTokenAccount(
			provider.connection,
			owner,
			minterNft1,
			owner.publicKey
		)

		await mintTo(
			provider.connection,
			owner,
			minterNft1,
			tokenAccount1,
			owner.publicKey,
			1
		)

		tokenAccount2 = await createAssociatedTokenAccount(
			provider.connection,
			owner,
			minterNft2,
			owner.publicKey
		)

		await mintTo(
			provider.connection,
			owner,
			minterNft2,
			tokenAccount2,
			owner.publicKey,
			1
		)

		tokenAccount3 = await createAssociatedTokenAccount(
			provider.connection,
			owner,
			minterNft3,
			owner.publicKey
		)

		await mintTo(
			provider.connection,
			owner,
			minterNft3,
			tokenAccount3,
			owner.publicKey,
			1
		)

		// let balance1 = await provider.connection.getTokenAccountBalance(
		// 	tokenAccount1
		// );
		// console.log("balance1", balance1);
	})

	it("Is initialized!", async () => {
		owner = await createKeypairFromFile(__dirname + "/../../../../tungleanh/.config/solana/id.json");
		// const [state_pda, _] = PublicKey.findProgramAddressSync([Buffer.from("state")], program.programId)
		const state_account = await createKeypairFromFile(__dirname + "/../../../my-solana-wallet/state_account.json");
		console.log(await provider.connection.getBalance(owner.publicKey));
		console.log(await provider.connection.getBalance(state_account.publicKey));
		console.log(await getMinimumBalanceForRentExemptAccount(provider.connection));

		await program.methods
			.initState(new BN(1000000))
			.accounts({
				systemProgram: SystemProgram.programId,
				stateAccount: state_account.publicKey,
				user: owner.publicKey,
			})
			.signers([state_account])
			.rpc();

		let state = (await program.account.state.fetch(state_account.publicKey)).initialized;
		expect(state).to.be.equal(true);

		const [pda, bump] = PublicKey.findProgramAddressSync(
			[Buffer.from("auth")],
			program.programId
		)

		let tokenAccountOfProgram1 = await getOrCreateAssociatedTokenAccount(
			provider.connection,
			owner,
			minterNft1,
			pda,
			true
		)

		let tokenAccountOfProgram2 = await getOrCreateAssociatedTokenAccount(
			provider.connection,
			owner,
			minterNft2,
			pda,
			true
		)

		let tokenAccountOfProgram3 = await getOrCreateAssociatedTokenAccount(
			provider.connection,
			owner,
			minterNft3,
			pda,
			true
		)

		let price = 100000000
		await program.methods
			.createMarketItem(new BN(price))
			.accounts({
				stateAccount: state_account.publicKey,
				to: tokenAccountOfProgram1.address,
				tokenProgram: TOKEN_PROGRAM_ID,
				user: buyer.publicKey,
				from: tokenAccount1,
			}).signers([buyer]).rpc();

		await program.methods
			.createMarketItem(new BN(price))
			.accounts({
				stateAccount: state_account.publicKey,
				to: tokenAccountOfProgram2.address,
				tokenProgram: TOKEN_PROGRAM_ID,
				user: owner,
				from: tokenAccount2,
			}).signers([owner]).rpc();

		await program.methods
			.createMarketItem(new BN(price))
			.accounts({
				stateAccount: state_account.publicKey,
				to: tokenAccountOfProgram3.address,
				tokenProgram: TOKEN_PROGRAM_ID,
				user: owner,
				from: tokenAccount3,
			}).signers([owner]).rpc();

		// let new_state = await program.account.state.fetch(state_account.publicKey);
		// expect(new_state.map.length).to.be.equal(3);
		// expect(new_state.itemIds.toNumber()).to.be.equals(3);
		
		// let tokenAccountOfBuyer = await createAssociatedTokenAccount(
		// 	provider.connection,
		// 	buyer,
		// 	minterNft1,
		// 	buyer.publicKey
		// )
		// await program.methods
		// 	.purchaseSale(
		// 		new BN(price),
		// 		new BN(0),
		// 		bump
		// 	)
		// 	.accounts({
		// 		stateAccount: state_account.publicKey,
		// 		tokenProgram: TOKEN_PROGRAM_ID,
		// 		user: buyer.publicKey,
		// 		fromTokenAccount: tokenAccountOfProgram1.address,
		// 		toTokenAccount: tokenAccountOfBuyer,
		// 		auth: pda,
		// 		seller: owner.publicKey,
		// 	}).signers([buyer]).rpc();
		// new_state = (await program.account.state.fetch(state_account.publicKey));
		// expect(new_state.itemSold.toNumber()).to.equal(1);

		let buyerTokenAddress1 = await anchor.utils.token.associatedAddress({
			mint: minterNft1,
			owner: buyer.publicKey
		});
		let buyerTokenAddress2 = await anchor.utils.token.associatedAddress({
			mint: minterNft2,
			owner: buyer.publicKey
		});
		let buyerTokenAddress3 = await anchor.utils.token.associatedAddress({
			mint: minterNft3,
			owner: buyer.publicKey
		});
		// await program.methods
		// 	.createGacha(
		// 		3,
		// 		bump
		// 	)
		// 	.accounts({
		// 		stateAccount: state_account.publicKey,
		// 		tokenProgram: TOKEN_PROGRAM_ID,
		// 		user: buyer.publicKey,
		// 		auth: pda,
		// 		seller: owner.publicKey,
		// 		associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
		// 		systemProgram: SystemProgram.programId,
		// 		owner: owner.publicKey // program owner
		// 	})
		// 	.signers([buyer])
		// 	.remainingAccounts([
		// 		{ pubkey: minterNft1, isWritable: true, isSigner: false }, // minter nft
		// 		{ pubkey: tokenAccountOfProgram1.address, isWritable: true, isSigner: false }, // from 
		// 		{ pubkey: buyerTokenAddress1, isWritable: true, isSigner: false }, // to
		// 		{ pubkey: minterNft2, isWritable: true, isSigner: false },
		// 		{ pubkey: tokenAccountOfProgram2.address, isWritable: true, isSigner: false },
		// 		{ pubkey: buyerTokenAddress2, isWritable: true, isSigner: false },
		// 		{ pubkey: minterNft3, isWritable: true, isSigner: false },
		// 		{ pubkey: tokenAccountOfProgram3.address, isWritable: true, isSigner: false },
		// 		{ pubkey: buyerTokenAddress3, isWritable: true, isSigner: false },
		// 	])
		// 	.rpc();

		await program.methods
			.gacha(
				3,
				new BN(100000000),
				new BN(1000),
				bump
			)
			.accounts({
				stateAccount: state_account.publicKey,
				tokenProgram: TOKEN_PROGRAM_ID,
				user: buyer.publicKey,
				auth: pda,
				seller: owner.publicKey,
				associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
				systemProgram: SystemProgram.programId,
			})
			.signers([buyer])
			.remainingAccounts([
				{ pubkey: minterNft1, isWritable: true, isSigner: false }, // minter nft
				{ pubkey: tokenAccountOfProgram1.address, isWritable: true, isSigner: false }, // from 
				{ pubkey: buyerTokenAddress1, isWritable: true, isSigner: false }, // to
				{ pubkey: minterNft2, isWritable: true, isSigner: false },
				{ pubkey: tokenAccountOfProgram2.address, isWritable: true, isSigner: false },
				{ pubkey: buyerTokenAddress2, isWritable: true, isSigner: false },
				{ pubkey: minterNft3, isWritable: true, isSigner: false },
				{ pubkey: tokenAccountOfProgram3.address, isWritable: true, isSigner: false },
				{ pubkey: buyerTokenAddress3, isWritable: true, isSigner: false },
			])
			.rpc();

		// console.log('//////////////////////////');
		// let t = await provider.connection.getTransaction(tx, {
		// 	commitment: "confirmed",
		// 	maxSupportedTransactionVersion: 0
		// });
		// console.log('//////////////////////////');
		// const [key, , buffer] = getReturnLog(t);
		// const reader = new borsh.BinaryReader(buffer);
		// console.log("reader:", reader.readArray(() => reader.readU128()));

		// let balance1 = await provider.connection.getTokenAccountBalance(
		// 	buyerTokenAddress1
		// );
		// console.log("balance1", balance1);
		// let balance2 = await provider.connection.getTokenAccountBalance(
		// 	buyerTokenAddress2
		// );
		// console.log("balance2", balance2);
		// let balance3 = await provider.connection.getTokenAccountBalance(
		// 	buyerTokenAddress3
		// );
		// console.log("balance3", balance3);
	});
});

const getReturnLog = (confirmedTransaction) => {
	const prefix = "Program return: ";
	console.log(confirmedTransaction);

	let log = confirmedTransaction.meta.logMessages.find((log) =>
		log.startsWith(prefix)
	);
	log = log.slice(prefix.length);
	const [key, data] = log.split(" ", 2);
	const buffer = Buffer.from(data, "base64");
	return [key, data, buffer];
};