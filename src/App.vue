<script setup lang="ts">
import { ref } from "vue";
// import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { open } from "@tauri-apps/plugin-dialog";
import { save } from "@tauri-apps/plugin-dialog";
import { Command } from "@tauri-apps/plugin-shell";
import { appCacheDir, join, basename} from '@tauri-apps/api/path';
import { copyFile, exists, readDir } from "@tauri-apps/plugin-fs";
import { mkdir, BaseDirectory, remove, rename, readFile, writeFile } from '@tauri-apps/plugin-fs';
import { getMatches } from "@tauri-apps/plugin-cli";
import { exit } from '@tauri-apps/plugin-process';

const greetMsg = ref("");
// const name = ref("");
const isRunning = ref(false);
const cacheText = ref("");

const ciaPath = ref("");
const romPath = ref("");
const patchPath = ref("");
const savePath = ref("");
const patchGen2Code = ref(false);
const isKoreanVC = ref(false);

const subFolderName = "extracted"
const replaceFolderName = "patches"

const pattern = new Uint8Array([
  0x00, 0x00, 0x00, 0x00,
  0x10, 0x17, 0x00, 0x00,
  0x27, 0x17, 0x00, 0x00,
  0x26, 0x17, 0x00, 0x00,
]);

const patternKorean = new Uint8Array([
  0x00, 0x00, 0x00, 0x00,
  0x25, 0x17, 0x00, 0x00,
  0x36, 0x17, 0x00, 0x00,
  0x35, 0x17, 0x00, 0x00,
]);

const newPattern = new Uint8Array([
  0x10, 0x17, 0x00, 0x00,
  0x10, 0x17, 0x00, 0x00,
  0x10, 0x17, 0x00, 0x00,
  0x10, 0x17, 0x00, 0x00,
]);

const gen2Msg:string = "由于Gen2宝可梦VC某些兼容性原因，CKN·DMG·口袋群星·Tom_C汉化的美版金·银·水晶，即便使用联机补丁，也无法正常和原版Gen1 Gen2连接，表现为无法搜索到房间。\n\n启用房间码修改会替换VC程序的搜索的房间码，替换成和欧美版Gen1房间码相同，这样可以实现时光胶囊。\n\n这个修改仅支持欧美韩版Gen2 VC。无法支持日版Gen2 VC。Gen 1 VC不需要此修改。也支持修改韩版VC与欧美版Gen 1进行时光胶囊。\n\n支持修改美版汉化版Gen2 VC，也可以修改原欧美版Gen2 VC，让其支持和修改过的汉化版Gen2 VC联机，原版VC修改后将不再支持和原版Gen2 VC联机。";

// async function greet() {
//   // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
//   greetMsg.value = await invoke("greet", { name: name.value });
// }



interface CliArg {
  value: string | boolean | string[] | null;
  occurrences: number;
}

async function handleCli() {
  const matches = await getMatches();
  const args = matches.args as Record<string, CliArg>;

  const cia = args.cia?.value;
  const save = args.save?.value;
  const patch = args.patch?.value;
  const rom = args.rom?.value;

  const gen2 = args.gen2patch?.value === true;
  const korean = args.iskorean?.value === true;

  if (typeof cia === "string" && typeof save === "string") {
    await runMakerom(
      cia,
      typeof patch === "string" ? patch : "",
      typeof rom === "string" ? rom : "",
      save,
      gen2,
      korean
    );
    await exit(0);
  }
}
handleCli();

function findPatternOffsets(data: Uint8Array,pattern: Uint8Array): number[] {
  const offsets: number[] = [];

  for (let i = 0; i <= data.length - pattern.length; i++) {
    let ok = true;

    for (let j = 0; j < pattern.length; j++) {
      if (data[i + j] !== pattern[j]) {
        ok = false;
        break;
      }
    }

    if (ok) {
      offsets.push(i);
    }
  }

  return offsets;
}

function replaceAt(
  data: Uint8Array,
  offset: number,
  replacement: Uint8Array
) {
  for (let i = 0; i < replacement.length; i++) {
    data[offset + i] = replacement[i];
  }
}

async function copyUserFileToCache(subFolderPath:string, userFilePath: string) {
  const fileName = await basename(userFilePath);
  await copyFile(userFilePath, subFolderPath+`${fileName}`, {
    toPathBaseDir: BaseDirectory.AppCache,
  });
  return fileName;
}

async function findFilesIn(subdir:string ,ext:string ) {
  const entries = await readDir(subdir, {
    baseDir: BaseDirectory.AppCache
  });

  const outputFiles: string[] = [];
  function walk(items: any[]) {
    for (const item of items) {
      if (item.children) {
        walk(item.children);
      } else {
        if (ext != ""){
          if (item.name?.endsWith(ext)) {
            outputFiles.push(item.name);
          }
        }else{
           outputFiles.push(item.name);
        }
      }
    }
  }
  walk(entries);
  return outputFiles;
}

function sortNcchFiles(files: string[]) {
  const getIndex = (path: string) => {
    const fileName = path.split(/[\\/]/).pop() ?? path;
    const match = fileName.match(/\.(\d+)\.[0-9a-fA-F]+\.ncch$/);
    return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
  };
  return files.sort((a, b) => getIndex(a) - getIndex(b));
}

async function createTmpFolder(){
  try {
    // Recursively delete a directory in the AppData folder
    if (await exists(subFolderName, { baseDir: BaseDirectory.AppCache})){
      await remove(subFolderName, { 
        baseDir: BaseDirectory.AppCache,
        recursive: true 
      });
    }

    await mkdir(subFolderName, { 
      baseDir: BaseDirectory.AppCache,
      recursive: true 
    });

    if (await exists(replaceFolderName, { baseDir: BaseDirectory.AppCache})){
      await remove(replaceFolderName, { 
        baseDir: BaseDirectory.AppCache,
        recursive: true 
      });
    }

    await mkdir(replaceFolderName, { 
      baseDir: BaseDirectory.AppCache,
      recursive: true 
    });

    console.log("Directory created successfully!");
  } catch (error) {
    let msg: string;
    if (error instanceof Error) {
      msg = error.message; // safe access
    } else {
      msg = String(error); // fallback for non-Error types (like strings or null)
    }
    await message(msg, {
    title: "创建目录失败",
    kind: "error"});
    return false;
  }
  return true;
}

async function showErrorAlert(title:string,error:unknown){
  let msg: string;
  if (error instanceof Error) {
    msg = error.message; // safe access
  } else {
    msg = String(error); // fallback for non-Error types (like strings or null)
  }
  await message(msg, {
  title: title,
  kind: "error"});
}

async function runMakeromForm(){
  await runMakerom(ciaPath.value,patchPath.value,romPath.value,savePath.value,patchGen2Code.value,isKoreanVC.value);
}

async function runMakerom(ciafilePath:string,patchFilePath:string,romFilePath:string,saveFilePath:string,patchGen2CodeBool:boolean,isKoreanVCBool:boolean) {
  // const cachePath = await appCacheDir();
  isRunning.value = true;
  greetMsg.value = "正在创建临时目录...";
  if (!await createTmpFolder()){
    isRunning.value = false;
    return;
  }

  try {
    await copyUserFileToCache(`${subFolderName}/`,ciafilePath);
    console.log("File Copied successfully!");
  } catch (error) {
    showErrorAlert("复制失败",error);
    isRunning.value = false;
    return;
  }

  const applyingPatch = patchFilePath.trim() != "";
  if (applyingPatch){
    try {
      await copyUserFileToCache(`${replaceFolderName}/`,patchFilePath);
      console.log("File Copied successfully!");
    } catch (error) {
      showErrorAlert("复制失败",error);
      isRunning.value = false;
      return;
    }
  }

  const applyingRom = romFilePath.trim() != "";
  if (applyingRom){
    try {
      await copyUserFileToCache(`${replaceFolderName}/`,romFilePath);
      console.log("File Copied successfully!");
    } catch (error) {
      showErrorAlert("复制失败",error);
      isRunning.value = false;
      return;
    }
  }

  greetMsg.value = "正在解密并提取CIA...";
  const appCachePath = await appCacheDir();
  cacheText.value = appCachePath;
  const ciaFileName = await basename(ciafilePath);
  const ciaFullPath = await join(appCachePath, subFolderName, ciaFileName);
  const commandCtrdecrypt = Command.sidecar("binaries/ctrdecrypt", [ciaFullPath]);
  const res = await commandCtrdecrypt.execute();
  console.log("ctrdecrypt");
  console.log(res.stdout);
  

  const ncchfiles = sortNcchFiles(await findFilesIn(subFolderName,".ncch"));
  if (ncchfiles.length < 1){
    await message("cia 中未找到 ncch，可能是解压失败", {
    title: "没找到 Ncch",
    kind: "error"});
    isRunning.value = false;
    return;
  }
  console.log(ncchfiles);

  const ncch0FilePathOld = `${await join(appCachePath, subFolderName, ncchfiles[0])}`;
  const ncch0FilePathNew = `${await join(appCachePath, subFolderName, "output0.ncch")}`;
  const ncchHeaderPath = `${await join(appCachePath, subFolderName, "ncchheader.bin")}`;
  const exHeaderPath = `${await join(appCachePath, subFolderName, "exheader.bin")}`;
  const logoPath = `${await join(appCachePath, subFolderName, "logo.bcma.lz")}`;
  const plainPath = `${await join(appCachePath, subFolderName, "plain.bin")}`;
  const exefsPathOld = `${await join(appCachePath, subFolderName, "exefs.bin")}`;
  const romfsPathOld = `${await join(appCachePath, subFolderName, "romfs.bin")}`;
  const exefsPathNew = `${await join(appCachePath, subFolderName, "_exefs.bin")}`;
  const romfsPathNew = `${await join(appCachePath, subFolderName, "_romfs.bin")}`;
  const exefsDirOld = `${await join(appCachePath, subFolderName, "exefs")}`;
  const romfsDirOld = `${await join(appCachePath, subFolderName, "romfs")}`;
  const exefsHeatherPath = `${await join(appCachePath, subFolderName, "exefsheader.bin")}`;
  const outputCiaPath = `${await join(appCachePath, replaceFolderName, "output.cia")}`;

  const command3dstool1 = Command.sidecar("binaries/3dstool", ["-xvtf", "cxi", ncch0FilePathOld, "--header", ncchHeaderPath, "--exh", exHeaderPath, "--logo", logoPath, "--plain", plainPath, "--exefs", exefsPathOld, "--romfs", romfsPathOld]);
  const res1 = await command3dstool1.execute();
  console.log(res1.stdout);
  // showAlert("title",res1.stdout);

  const command3dstool2 = Command.sidecar("binaries/3dstool", ["-xvtf", "romfs", romfsPathOld, "--romfs-dir", romfsDirOld]);
  await command3dstool2.execute();

  const commandctrtool1= Command.sidecar("binaries/ctrtool", ["--exefsdir="+exefsDirOld, "--decompresscode", exefsPathOld]);
  await commandctrtool1.execute();

  const command3dstool3 = Command.sidecar("binaries/3dstool", ["-xvtf", "exefs", exefsPathOld, "--header", exefsHeatherPath]);
  await command3dstool3.execute();

  try{
    await rename(`${subFolderName}/exefs/banner.bin`, `${subFolderName}/exefs/banner.bnr`, { 
      oldPathBaseDir: BaseDirectory.AppCache,
      newPathBaseDir: BaseDirectory.AppCache 
    });

    await rename(`${subFolderName}/exefs/icon.bin`, `${subFolderName}/exefs/icon.icn`, { 
      oldPathBaseDir: BaseDirectory.AppCache,
      newPathBaseDir: BaseDirectory.AppCache 
    });
  }catch (error){
    await showErrorAlert("Rename error", error);
    isRunning.value = false;
    return;
  }

  if (applyingRom){
    greetMsg.value = "正在应用ROM...";
    const romNames = await findFilesIn(`${subFolderName}/romfs/rom`,"");
    if (romNames.length < 1){
      await message("cia 解压失败或者 cia 中未找到 rom", {
      title: "没找到 ROM",
      kind: "error"});
      isRunning.value = false;
      return;
    }
    const targetRomName = romNames[0];
    const sourceRomName = await basename(romFilePath);
    await copyFile(`${replaceFolderName}/${sourceRomName}`, `${subFolderName}/romfs/rom/${targetRomName}`, {
      fromPathBaseDir: BaseDirectory.AppCache,
      toPathBaseDir: BaseDirectory.AppCache,
    });
  }

  if (applyingPatch){
    greetMsg.value = "正在应用补丁...";
    const romNames = await findFilesIn(`${subFolderName}/romfs/rom`,"");
    if (romNames.length < 1){
      await message("cia 解压失败或者 cia 中未找到 rom", {
      title: "没找到 ROM",
      kind: "error"});
      isRunning.value = false;
      return;
    }
    const targetRomName = romNames[0];
    const sourcePatchName = await basename(patchFilePath);
    await copyFile(`${replaceFolderName}/${sourcePatchName}`, `${subFolderName}/romfs/${targetRomName}.patch`, {
      fromPathBaseDir: BaseDirectory.AppCache,
      toPathBaseDir: BaseDirectory.AppCache,
    });
  }

  if (patchGen2CodeBool){
    try{
      greetMsg.value = "正在替换房间码...";
      const data = await readFile(`${subFolderName}/exefs/code.bin`, {
        baseDir: BaseDirectory.AppCache,
      });

      const searchPattern = isKoreanVCBool? patternKorean : pattern;
      const offsets = findPatternOffsets(data, searchPattern);
      if (offsets.length != 1){
        await message(`找到${offsets.length}个可替换的位置\n当前仅支持替换1个。\n将跳过房间码修改。`, {
        title: '搜索房间码失败',
        kind: "error"});
      }else{
        replaceAt(data, offsets[0], newPattern);
      }
      await writeFile(`${subFolderName}/exefs/code.bin`, data, {
        baseDir: BaseDirectory.AppCache,
      });
    }
    catch (error){
      showErrorAlert("修改房间码出错",error);
    }
  }

  greetMsg.value = "正在重新打包CIA...";
  const command3dstool4 = Command.sidecar("binaries/3dstool", ["-cvtf", "romfs", romfsPathNew, "--romfs-dir", romfsDirOld]);
  await command3dstool4.execute();

  const command3dstool5 = Command.sidecar("binaries/3dstool", ["-cvtf", "exefs", exefsPathNew, "--exefs-dir", exefsDirOld, "--header", exefsHeatherPath]);
  await command3dstool5.execute();


  const command3dstool6 = Command.sidecar("binaries/3dstool", ["-cvtf", "cxi", ncch0FilePathNew, "--header", ncchHeaderPath, "--exh", exHeaderPath, "--logo", logoPath, "--plain", plainPath, "--exefs", exefsPathNew, "--romfs", romfsPathNew]);
  await command3dstool6.execute();

  var args = ["-f", "cia", "-ignoresign", "-target", "p", "-o", outputCiaPath, "-i", `${ncch0FilePathNew}:0:0`];

  for (let i = 1; i < ncchfiles.length; i++){
    args.push("-i");
    args.push(`${await join(appCachePath, subFolderName, ncchfiles[i])}:${i}:${i}`);
  }

  console.log(args);
  const command3dstool7 = Command.sidecar("binaries/makerom", args);
  const result8 = await command3dstool7.execute();

  // showAlert("title",result8.stdout);
// ./makerom -f cia -ignoresign -target p -o output.cia -i extracted/input.0.ncch:0:0 -i "$ciafilenoextension.1.ncch":1:1
  // showAlert("title",cachePath);
  // console.log("stdout:", output.stdout);
  console.log("stderr:", result8.stderr);
  // console.log("code:", output.code);
  try{
    await copyFile(outputCiaPath,saveFilePath);
  }
  catch (error){
    showErrorAlert("生成 cia 失败", "可能是打包过程中存在问题\n" + result8.stderr);
    isRunning.value = false;
    return;
  }
  greetMsg.value = "完成！";
  isRunning.value = false;
  return;
}

async function selectExtensionFile(extension:string) {
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: extension + " 文件",
        extensions: [extension]
      }
    ]
  });
  if (selected && typeof selected === "string") {
    if (extension == 'cia'){
      ciaPath.value = selected;
    }else if (extension == 'patch'){
      patchPath.value = selected;
    }    
  }
}

async function selectAnyFile() {
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: "任意文件",
        extensions: []
      }
    ]
  });
  if (selected && typeof selected === "string") {
    romPath.value = selected;
  }
}



async function selectSavePath() {
  const path = await save({
    filters: [
      {
        name: "CIA 文件",
        extensions: ["cia"]
      }
    ]
  });
  if (path) {
    savePath.value = path;
  }
}

async function showAlert(title:string, msg:string) {
  await message(msg, {
    title: title,
    kind: "info"
  });
}



</script>

<template>
  <main class="container">
    <h2>3DS cia VC ROM/联机补丁替换工具 v0.1.0 Tom_C</h2>
<!-- 
    <div class="row">
      <a href="https://vite.dev" target="_blank">
        <img src="/vite.svg" class="logo vite" alt="Vite logo" />
      </a>
      <p></p>
      <a href="https://tauri.app" target="_blank">
        <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
      </a>
      <p></p>
      <a href="https://vuejs.org/" target="_blank">
        <img src="./assets/vue.svg" class="logo vue" alt="Vue logo" />
      </a>
    </div> -->
    <!-- <p>Click on the Tauri, Vite, and Vue logos to learn more.</p> -->

    <form class="row" @submit.prevent="selectExtensionFile('cia')">
      <input id="greet-input" v-model="ciaPath" placeholder="请选择要拆包的 .cia 文件" />
      <button type="submit">选择CIA</button>
    </form>
    <form class="row" @submit.prevent="selectAnyFile">
      <input id="greet-input" v-model="romPath" placeholder="请选择要替换的 ROM 文件（可选）" />
      <button type="submit">选择ROM</button>
    </form>
    <form class="row" @submit.prevent="selectExtensionFile('patch')">
      <input id="greet-input" v-model="patchPath" placeholder="请选择要替换 .patch 补丁文件（可选）" />
      <button type="submit">选择补丁</button>
    </form>
    <form class="row" @submit.prevent="selectSavePath">
      <input id="greet-input" v-model="savePath" placeholder="请选择 .cia 保存位置" />
      <button type="submit">选择保存位置</button>
    </form>
    <div class="row">
        <label class="checkbox">
          <input type="checkbox" v-model="patchGen2Code" />
          <span class="box"></span>
          <span>修改宝可梦Gen2 VC房间搜索码</span>
        </label>
        <label class="checkbox">
          <input type="checkbox" v-model="isKoreanVC" :disabled="!patchGen2Code" />
          <span class="box"></span>
          <span>原始cia是韩版</span>
        </label>
        <button type="button" @click="showAlert('关于 Gen 2 宝可梦房间码搜索',gen2Msg)">这是什么？</button>
    </div>
    <form class="row" @submit.prevent="runMakeromForm">
      <button type="submit" :disabled="isRunning">开始</button>
    </form>
    <div class="row">
      {{ greetMsg }}
      临时目录：{{ cacheText }}
    </div>
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}
.checkbox input:disabled + .box {
  background: #ccc;
  border-color: #ccc;
  cursor: not-allowed;
}

.checkbox input:disabled ~ span {
  opacity: 0.5;
}

.checkbox {
  margin: 1% ;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  user-select: none;
}
.checkbox input {
  display: none;
}
.box {
  width: 20px;
  height: 20px;
  border: 2px solid #aaa;
  border-radius: 6px;
  position: relative;
  transition: all 0.2s;
}
.checkbox input:checked + .box {
  background: #396cd8;
  border-color: #396cd8;
}
.checkbox input:checked + .box::after {
  content: "";
  position: absolute;
  left: 6px;
  top: 2px;
  width: 5px;
  height: 10px;
  border: solid white;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
}

</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 2vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  /* margin-top: 10px;
   */
  width: 100%;
  margin: 1% auto;

  display: flex;
  justify-content: center;
}

input {
  flex: 1;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 10px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>