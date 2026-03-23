import { Command, Child } from '@tauri-apps/plugin-shell';
import { invoke } from '@tauri-apps/api/core';
import { resourceDir, appLogDir, appDataDir, appLocalDataDir, documentDir } from '@tauri-apps/api/path';
import { platform } from '@tauri-apps/plugin-os';

const VITE_APP_HUB_LISTEN = import.meta.env.VITE_APP_HUB_LISTEN;
export default class ShellService {
	async getDB () {
		const dbpath = localStorage.getItem("DB_PATH");
		if(dbpath){
			return dbpath
		}
		const appDataDirPath = await documentDir();
		return `${appDataDirPath}/ztmdb`
	}
	async startPipy (callError){
		const pm = window.__TAURI_OS_PLUGIN_INTERNALS__ && platform();
		console.log(pm)
		const port = 6789;
		let resourceDirPath = '';
		if(pm == "android" ){
			let resourceDirPath = await documentDir();//appLocalDataDir();
			console.log(resourceDirPath)
			const args = [
				"./main",
				"--pipy",
				"repo://ztm/agent",
				"--args",
				`--listen`,
				`${port}`,
				`--data`,
				`${resourceDirPath}/ztmdb`,
				"--pipy-options",
				`--log-file=${resourceDirPath}/ztm.log`,
			];
			console.log(args)
			const filePath = await appLocalDataDir();
			invoke('pipylib', {
				lib:`${filePath}/files/libztm.so`,
				argv: args,
				argc: args.length
			}).then((res)=>{
				// store.commit('account/setPid', 1);
				console.log(`[pipylib]Result: ${res}`);
			});
		} else if (!!pm && pm!='web') {
			let resourceDirPath = await documentDir();//resourceDir();
			let dbPath = await this.getDB();
			const args = [
				"--pipy",
				"repo://ztm/agent",
				"--args",
				`--listen`,
				`${port}`,
				`--data`,
				dbPath,
				"--pipy-options",
				`--log-file=${resourceDirPath}/ztm.log`,
			];
			console.log(`[starting pipy:${args}]`);
			const command = Command.sidecar("bin/ztm", args);
			command.on('close', data => {
				// console.log("[close]");
				// console.log(data);
				// store.commit('account/pushLog', {level:'Info',msg:`pipy pause with code ${data.code} and signal ${data.signal}`});
			});
			command.stdout.on('data', line => {
				console.log("[stdout]",line);
				// console.log(line);
				// store.commit('account/pushLog', {level:'Info',msg:line});
			});
			command.stderr.on('data', line => {
				console.log("[stderr]",line);
				// console.log(line);
				// store.commit('account/pushLog', {level:'Error',msg:line});
				callError(line);
			});
			command.on('error', error => {
				console.log("[error]",error);
				// store.commit('account/pushLog', {level:'Error',msg:error});
				callError(error);
			});
			let child = await command.spawn();
		}
	}
}
