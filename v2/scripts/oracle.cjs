/* Execute the actual bundled own TSX and locked libraries inside a closed VM.
 * No fs callback is invoked until pathname/fd ownership has been checked. */
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const QA=path.resolve(__dirname,'../.qa'),root=path.join(QA,'oracle',crypto.randomUUID());fs.mkdirSync(root,{recursive:true});
const fds=new Set(),accesses=[];
function checked(p){
 if(typeof p==='number'){assert(fds.has(p),'foreign fd');return p;}
 if(p instanceof URL)p=require('node:url').fileURLToPath(p);
 assert(typeof p==='string'||Buffer.isBuffer(p),'non-path');p=path.resolve(String(p));
 assert(p===QA||p.startsWith(QA+path.sep),'outside QA: '+p);
 let current=QA;for(const n of path.relative(QA,p).split(path.sep).filter(Boolean)){current=path.join(current,n);try{assert(!fs.lstatSync(current).isSymbolicLink(),'symlink '+current);}catch(e){if(e.code==='ENOENT')break;throw e;}}
 accesses.push(p);return p;
}
const guarded={constants:fs.constants,Stats:fs.Stats};
const pathMethods=['readFile','writeFile','appendFile','stat','lstat','readdir','mkdir','rmdir','unlink','chmod','utimes','access','realpath','truncate'];
for(const name of pathMethods){
 guarded[name+'Sync']=(p,...args)=>{const q=typeof p==='string'?path.resolve(p):null;if((name==='stat'||name==='lstat')&&q&&(QA.startsWith(q+path.sep)||q===path.parse(q).root))return fs.statSync(QA);p=checked(p);return fs[name+'Sync'](p,...args)};
 guarded[name]=(p,...args)=>{p=checked(p);return fs[name](p,...args)};
}
// Existence checks used by AdmZip's mkdir walker get synthetic parent facts,
// without probing any parent outside the QA root.
guarded.existsSync=p=>{p=path.resolve(String(p));if(QA.startsWith(p+path.sep)||p===path.parse(p).root)return true;return fs.existsSync(checked(p));};
guarded.exists=(p,cb)=>cb(guarded.existsSync(p));
guarded.openSync=(p,...args)=>{let fd=fs.openSync(checked(p),...args);fds.add(fd);return fd;};
guarded.open=(p,...args)=>{p=checked(p);const cb=args.pop();fs.open(p,...args,(e,fd)=>{if(!e)fds.add(fd);cb(e,fd);});};
for(const n of ['read','write','fstat','ftruncate','fsync','fdatasync','fchmod','futimes']){
 guarded[n+'Sync']=(fd,...a)=>fs[n+'Sync'](checked(fd),...a);
 guarded[n]=(fd,...a)=>fs[n](checked(fd),...a);
}
guarded.closeSync=fd=>{checked(fd);fs.closeSync(fd);fds.delete(fd);};
guarded.close=(fd,cb)=>{checked(fd);fs.close(fd,e=>{fds.delete(fd);cb(e);});};
for(const n of ['rename','copyFile','link']){guarded[n+'Sync']=(a,b,...rest)=>fs[n+'Sync'](checked(a),checked(b),...rest);guarded[n]=(a,b,...rest)=>fs[n](checked(a),checked(b),...rest);}
for(const n of ['createReadStream','createWriteStream'])guarded[n]=(p,options={})=>{assert(!options.fs,'custom fs');p=checked(p);if(options.fd!==undefined)checked(options.fd);return fs[n](p,{...options,fs:guarded});};
const promises={};for(const n of pathMethods)promises[n]=async(p,...a)=>fs.promises[n](checked(p),...a);
promises.open=async(p,...a)=>{
 const h=await fs.promises.open(checked(p),...a);fds.add(h.fd);const fd=h.fd;
 return new Proxy(h,{get(t,key){if(key==='fd')return fd;if(key==='then')return undefined;if(typeof t[key]!=='function')return t[key];return(...args)=>{checked(fd);if(key==='close')return t.close().finally(()=>fds.delete(fd));if(['stat','read','readFile','write','writeFile','appendFile','truncate','sync','datasync','chmod','utimes','createReadStream','createWriteStream','readableWebStream'].includes(key))return t[key](...args);throw Error('unapproved handle producer '+String(key));};}});
};
guarded.promises=promises;
const fsBoundary=new Proxy(guarded,{get(t,k){if(k==='__esModule')return false;if(k in t)return t[k];throw Error('unapproved fs API '+String(k));}});
let prefs={},selected=[],effects=[],markdown='',completed=false,errors=[];
const react={useState:()=>[undefined,x=>{markdown=typeof x==='function'?x(markdown):x;}],useEffect:cb=>effects.push(cb)};
const api={Detail:function(){},getPreferenceValues:()=>prefs,getSelectedFinderItems:async()=>selected.map(p=>({path:checked(p)})),showToast:async()=>({}),Toast:{Style:{Animated:'animated',Success:'success'}}};
const allowed=new Set(['path','crypto','zlib','util','events','stream','stream/web','stream/promises','buffer','assert','os','constants','url']);
function load(name){name=name.replace(/^node:/,'');if(name==='fs')return fsBoundary;if(name==='fs/promises')return promises;if(name==='tty')return{isatty:()=>false};if(name==='react')return react;if(name==='react/jsx-runtime')return{jsx:(type,props)=>({type,props}),jsxs:(type,props)=>({type,props})};if(name==='@raycast/api')return api;assert(allowed.has(name),'unapproved module '+name);return require('node:'+name);}
const sandbox={module:{exports:{}},exports:{},require:load,Buffer,Uint8Array,ArrayBuffer,DataView,TextDecoder,TextEncoder,ReadableStream,TransformStream,DecompressionStream,CompressionStream,AbortController,AbortSignal,DOMException,URL,setImmediate,clearImmediate,setTimeout,clearTimeout,console:{log(){},warn(){},error(...a){console.error('V1 diagnostic',...a)}},process:{stderr:{fd:-1,write(){}},platform:process.platform,version:process.version,versions:{node:process.versions.node},env:{},cwd:()=>root}};
sandbox.exports=sandbox.module.exports;
const context=vm.createContext(sandbox,{codeGeneration:{strings:false,wasm:false}});
new vm.Script(fs.readFileSync(path.join(QA,'oracle-bundle.cjs'),'utf8'),{filename:'actual-v1-bundle'}).runInContext(context,{timeout:3000});
const source=sandbox.module.exports;
process.on('unhandledRejection',e=>errors.push(String(e)));
const defaults={showMD5Log:true,enableVideo:true,enableAudio:false,enableImage:false,enableZipPassword:false,zipPassword:'1024'};
async function run(command,paths,override={}){
 prefs={...defaults,...override};selected=paths;effects=[];markdown='';errors=[];source[command]();assert.equal(effects.length,1);const callbacks=effects.splice(0);for(const cb of callbacks)cb();
 const end=Date.now()+5000;while(!String(markdown).includes('Completed!')&&!errors.length&&Date.now()<end)await new Promise(setImmediate);
 assert(!errors.length,errors.join(';'));assert(String(markdown).includes('Completed!'),'V1 callback did not complete');return markdown;
}
const original={};function put(n,b){const p=path.join(root,n);fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b);original[n]=crypto.createHash('sha256').update(b).digest('hex');return p;}
(async()=>{
 const checks=[];const mp4=Buffer.concat([Buffer.from('000000186674797069736f6d','hex'),Buffer.alloc(20)]),mark=Buffer.from('#$1024$#');
 const a=put('source/no extension',mp4);await run('modifyHash',[a]);assert.deepEqual(fs.readFileSync(a),Buffer.concat([mp4,mark]));checks.push('actual TSX modify marker and MD5');
 await run('restoreHash',[a]);assert.deepEqual(fs.readFileSync(a),mp4);checks.push('actual TSX restore one marker');
 fs.appendFileSync(a,Buffer.from('unrecognized#$1024$'));const old=fs.readFileSync(a);await run('restoreHash',[a]);assert.deepEqual(fs.readFileSync(a),old);checks.push('actual TSX restore preserves unrecognized bytes');
 const gif=put('source/image.mp4',Buffer.from('GIF89a00000000000000000000000000'));await run('modifyHash',[gif]);assert.equal(fs.readFileSync(gif).length,32);checks.push('actual TSX default image disabled');
 const folder=path.join(root,'zip-source');fs.mkdirSync(folder,{recursive:true});put('zip-source/one.txt',Buffer.from('one'));put('zip-source/sub/two',mp4);
 await run('zipCompress',[folder]);const archive=folder+'.zip';const zip=new source.Zip(checked(archive));assert.deepEqual(zip.readFile('zip-source/one.txt'),Buffer.from('one'));checks.push('actual TSX + locked AdmZip compress');
 const extractLog=await run('zipExtract',[archive]);if(extractLog.includes('Error'))console.error(extractLog);assert.deepEqual(fs.readFileSync(folder+' 2/zip-source/sub/two'),mp4);checks.push('actual TSX + locked AdmZip extract naming/collision');
 const media={};const signatures={bmp:'424d',ac3:'0b77',gif:'474946383961',jxr:'4949bc',jpg:'ffd8ffdb',flif:'464c4946',psd:'38425053',mpc7:'4d502b',mpc8:'4d50434b',aif:'464f524d',icns:'69636e73',mid:'4d546864',dsf:'44534420',flac:'664c6143',bpg:'425047fb',wv:'7776706b',ape:'4d414320',amr:'2321414d52',flv:'464c5601',it:'494d504d',xcf:'67696d702078636620',ktx:'ab4b5458203131bb0d0a1a0a',j2c:'ff4fff51',jxl:'ff0a',raf:'46554a4946494c4d4343442d524157',xm:'457874656e646564204d6f64756c653a',voc:'437265617469766520566f6963652046696c65',mp3:'fffb',aac:'fff1',mpg:'000001b321',ico:'00000100',cur:'00000200'};
 for(const [name,hex]of Object.entries(signatures))media[name]=Buffer.concat([Buffer.from(hex,'hex'),Buffer.alloc(100)]);
 for(const brand of ['isom','M4A ','M4B ','F4A ','F4B ','avif','heic','mif1','crx ','qt  ','3gp4'])media['brand-'+brand.trim()]=Buffer.concat([Buffer.from('0000001866747970','hex'),Buffer.from(brand),Buffer.alloc(30)]);
 for(const [name,sub]of [['avi','AVI '],['wav','WAVE'],['webp','WEBP'],['qcp','QLCM']])media[name]=Buffer.concat([Buffer.from('RIFF'),Buffer.alloc(4),Buffer.from(sub),Buffer.alloc(30)]);
 for(const [name,type]of [['opus','OpusHead'],['ogg','\x01vorbis'],['ogv','\x80theora'],['spx','Speex  '],['ogx','nothing!']])media[name]=Buffer.concat([Buffer.from('OggS'),Buffer.alloc(24),Buffer.from(type,'latin1'),Buffer.alloc(40)]);
 media.text=Buffer.from('plain text');media['bad-id3']=Buffer.from('ID3');media['utf16']=Buffer.from('fffe00410042','hex');media.png=Buffer.from('89504e470d0a1a0a0000000d4948445200000001000000010802000000000000000000000049444154','hex');media['big-tiff']=Buffer.from('49492b00000000000000000000000000','hex');media.tiff=Buffer.from('49492a00080000000000000000000000','hex');
 for(const [name,doctype]of [['webm','webm'],['mkv','matroska']])media[name]=Buffer.concat([Buffer.from('1a45dfa3','hex'),Buffer.from([0x80+3+doctype.length,0x42,0x82,0x80+doctype.length]),Buffer.from(doctype),Buffer.alloc(30)]);
 for(const [name,guid]of [['asf-audio','409e69f84d5bcf11a8fd00805f5c442b'],['asf-video','c0ef19bc4d5bcf11a8fd00805f5c442b']]){const h=Buffer.alloc(30);Buffer.from('3026b2758e66cf11a6d900aa0062ce6c','hex').copy(h);const object=Buffer.alloc(78);Buffer.from('9107dcb7b7a9cf118ee600c00c205365','hex').copy(object);object.writeBigUInt64LE(78n,16);Buffer.from(guid,'hex').copy(object,24);media[name]=Buffer.concat([h,object]);}
 media.mts=Buffer.alloc(200);media.mts[0]=media.mts[188]=0x47;media.bdav=Buffer.alloc(200);media.bdav[4]=media.bdav[196]=0x47;
 media['jls']=Buffer.from('ffd8fff700000000','hex');media['cr2']=Buffer.from('49492a00080000004352020000000000','hex');media['nef']=Buffer.from('49492a00080000001c00fe0000000000','hex');media['tiff-be']=Buffer.from('4d4d002a00000008000000000000','hex');
 media['orf']=Buffer.concat([Buffer.from('4949524f0800000018','hex'),Buffer.alloc(30)]);media['rw2']=Buffer.concat([Buffer.from('494955001800000088e774d8','hex'),Buffer.alloc(30)]);media['dwg']=Buffer.from('AC10240000000000');
 for(const brand of ['jp2 ','jpx ','jpm ','mjp2']){media['jp-'+brand.trim()]=Buffer.concat([Buffer.from('0000000c6a5020200d0a870a','hex'),Buffer.alloc(8),Buffer.from(brand),Buffer.alloc(30)]);}
 media['id3-mp3']=Buffer.concat([Buffer.from('49443304000000000000fffb','hex'),Buffer.alloc(30)]);media['bom-gif']=Buffer.concat([Buffer.from('efbbbf','hex'),Buffer.from('GIF89a000000')]);
 media['xml-utf16']=Buffer.from('fffe3c003f0078006d006c002000','hex');
 const evidence=[];
 for(const [name,bytes]of Object.entries(media)){const f=put('media/'+name,bytes);let type=await source.fileTypeFromFile(checked(f));evidence.push({name,path:f,base64:bytes.toString('base64'),category:type?.mime.split('/')[0]??null,type:type??null,sha256:crypto.createHash('sha256').update(bytes).digest('hex')});}
 // Prove the fs boundary exists before source callbacks and producers run.
 for(const fn of [()=>fsBoundary.readFileSync('/etc/passwd'),()=>fsBoundary.promises.open('/etc/passwd'),()=>fsBoundary.createReadStream('/etc/passwd')]){try{await fn();throw Error('guard escaped');}catch(e){assert.match(String(e),/outside QA/);}}
 assert.equal(fds.size,0,'source leaked fd');
 fs.writeFileSync(path.join(QA,'oracle-results.json'),JSON.stringify({checks,evidence,original,guardedAccesses:accesses.length,fsBoundary:'sync, promises, callbacks, FileHandle and streams guarded before callbacks'},null,2)+'\n');
 console.log('PASS actual V1 oracle',checks.length,'command checks;',evidence.length,'locked file-type fixtures;',accesses.length,'guarded fs accesses');
})().catch(e=>{console.error(e);process.exitCode=1;});
