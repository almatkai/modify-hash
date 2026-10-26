const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const V2=path.resolve(__dirname,'..'),QA=path.join(V2,'.qa'),ts=require(path.join(QA,'sourcecopy/node_modules/typescript'));
const base=path.join(V2,'evidence/host/src/renderer/src/raycast-api'),patched=path.join(QA,'hostpatch/src/renderer/src/raycast-api');
const checks=[];
function functions(file,names){
 const source=fs.readFileSync(file,'utf8'),ast=ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
 const declarations=ast.statements.filter(ts.isFunctionDeclaration).map(n=>n.getText(ast)).join('\n');
 const js=ts.transpileModule(declarations+'\nmodule.exports={'+names.join(',')+'}',{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX}}).outputText;
 const context=vm.createContext({module:{exports:{}},exports:{},require:n=>{assert.equal(n,'react/jsx-runtime');return {jsx:(type,props)=>({type,props}),jsxs:(type,props)=>({type,props})};}},{codeGeneration:{strings:false,wasm:false}});
 new vm.Script(js).runInContext(context,{timeout:1000});return {exports:context.module.exports,ast,source};
}
const form=functions(path.join(base,'form-runtime.tsx'),['collectFormItems','reconcileFormValues']).exports;
const after=functions(path.join(patched,'form-runtime.tsx'),['collectFormItems','reconcileFormValues','advanceFormDraft','admitFormSelection']);
const list=functions(path.join(base,'list-runtime.tsx'),['parseListRows']).exports;
const detail=functions(path.join(base,'detail-runtime.tsx'),['markdownFromNode']).exports;
const fields=[['paths','Form.TextArea','[]'],['showMD5Log','Form.Checkbox',true],['enableVideo','Form.Checkbox',true],['enableAudio','Form.Checkbox',false],['enableImage','Form.Checkbox',false],['enableZipPassword','Form.Checkbox',false],['zipPassword','Form.PasswordField','1024']].map(([id,type,value])=>({type,props:{id,title:id,value,actionId:'owned:field'}}));
const root={type:'Form',props:{draftRevision:0,requiresDraftRevision:true},children:fields};
let parsed=form.collectFormItems(root).map(n=>n.field);assert.equal(parsed.length,7);assert.equal(form.reconcileFormValues(parsed).showMD5Log,true);checks.push('pinned Form parser and default reconciliation');
const changes=JSON.parse(JSON.stringify(fields));changes[0].props.value='["/synthetic/A"]';
const updated=form.collectFormItems({...root,children:changes}).map(n=>n.field);assert.equal(form.reconcileFormValues(updated,{paths:'local-stale'},parsed).paths,'["/synthetic/A"]');checks.push('controlled Form fields accept server revisions');
let draft={owner:'owned',pending:null,revision:0,values:form.reconcileFormValues(parsed)};
for(const value of ['["/synthetic/A"]','["/synthetic/B"]','INVALID','["/synthetic/A"]'])draft=after.exports.advanceFormDraft(draft,'paths',value);
assert.equal(draft.revision,4);assert.equal(draft.values.showMD5Log,true);assert.equal(draft.values.paths,'["/synthetic/A"]');checks.push('patched full draft A B invalid A preserves all seven fields');
// Execute the actual patched nested update producer, with component boundaries
// captured. The callback receives the same complete draft the real parent sends.
let update,snapshot;
function visit(n){if(ts.isVariableDeclaration(n)&&n.name.getText(after.ast)==='updateValue')update=n.initializer.getText(after.ast);if(ts.isVariableDeclaration(n)&&n.name.getText(after.ast)==='snapshot')snapshot=n.initializer.getText(after.ast);ts.forEachChild(n,visit);}visit(after.ast);assert(update&&snapshot);
const calls=[],ctx=vm.createContext({completeDraft:true,values:form.reconcileFormValues(parsed),draftRef:{current:{owner:'owned',pending:null,revision:0,values:form.reconcileFormValues(parsed)}},advanceFormDraft:after.exports.advanceFormDraft,setValues(){},onChangeField:(id,value,values)=>calls.push({id,value,values})},{codeGeneration:{strings:false,wasm:false}});
let js=ts.transpileModule('const snapshot='+snapshot+';const updateValue='+update+';globalThis.updateValue=updateValue;globalThis.snapshot=snapshot;',{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
new vm.Script(js).runInContext(ctx,{timeout:1000});
for(const value of ['A','B','INVALID','A'])ctx.updateValue({id:'paths',actionId:'owned:field'},value);
ctx.updateValue({id:'enableImage',actionId:'owned:field'},true);
assert.deepEqual(calls.map(c=>c.values._draftRevision),[1,2,3,4,5]);assert.equal(ctx.snapshot().enableImage,true);assert.equal(Object.keys(ctx.snapshot()).length,8);checks.push('actual patched onChange and submit snapshot producers');
// Mount the entire transpiled pinned/patched React components against deterministic
// hook/JSX adapters. Exercise the actual textarea, checkbox, action-overlay,
// imperative-handle and useEffect callbacks, rather than reimplementing them.
function mount(file, exportName, initialProps, extraModules={}) {
 let slots=[],cursor=0,effects=[],dirty=false,tree,props=initialProps,ref={current:null};
 const equal=(a,b)=>a&&b&&a.length===b.length&&a.every((x,i)=>Object.is(x,b[i]));
 const react={
  forwardRef:fn=>fn,
  useRef(value){const i=cursor++;return slots[i]??(slots[i]={current:value});},
  useState(value){const i=cursor++;if(!(i in slots))slots[i]=typeof value==='function'?value():value;return [slots[i],next=>{slots[i]=typeof next==='function'?next(slots[i]):next;dirty=true;}];},
  useMemo(fn,deps){const i=cursor++;if(!slots[i]||!equal(slots[i].deps,deps))slots[i]={deps,value:fn()};return slots[i].value;},
  useCallback(fn,deps){return react.useMemo(()=>fn,deps);},
  useEffect(fn,deps){const i=cursor++;if(!slots[i]||!equal(slots[i].deps,deps)){slots[i]={deps};effects.push(fn);}},
  useImperativeHandle(target,fn){cursor++;if(target)target.current=fn();},
 };
 // Named imports call hooks without a receiver.
 react.useCallback=(fn,deps)=>react.useMemo(()=>fn,deps);
 const jsx=(type,props)=>({type,props});
 const modules={react,'react/jsx-runtime':{jsx,jsxs:jsx},'../../ui/primitives':{Hint:'Hint',HintBar:'HintBar',Kbd:'Kbd',ViewHeader:'ViewHeader'},'./detail-runtime':{parseNativeColorWheel:()=>null,NativeColorWheel:'NativeColorWheel'},...extraModules};
 const source=fs.readFileSync(file,'utf8');
 const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX}}).outputText;
 const context=vm.createContext({module:{exports:{}},exports:{},require:n=>{assert(n in modules,n);return modules[n];},window:{addEventListener(){},removeEventListener(){},clearTimeout(){},setTimeout(){throw Error('unexpected timer');}},HTMLTextAreaElement:class {}},{codeGeneration:{strings:false,wasm:false}});
 context.exports=context.module.exports;new vm.Script(js).runInContext(context,{timeout:1000});
 function render(next=props){props=next;for(let i=0;i<10;i++){cursor=0;effects=[];dirty=false;tree=context.module.exports[exportName](props,ref);for(const effect of effects)effect();if(!dirty)return tree;}throw Error('render loop');}
 function nodes(type,node=tree){if(!node||typeof node!=='object')return [];if(Array.isArray(node))return node.flatMap(n=>nodes(type,n));return [...(node.type===type?[node]:[]),...nodes(type,node.props?.children??null)];}
 render();return {render,nodes,ref,get props(){return props;}};
}
const clone=x=>JSON.parse(JSON.stringify(x));
const makeRoot=(owner='owned')=>({...clone(root),props:{...root.props,draftOwner:owner,selectionActionId:'select:1'}});
function formMount(invoke=()=>{},change=()=>{},owner='owned') {
 return mount(path.join(patched,'form-runtime.tsx'),'FormRuntime',{root:makeRoot(owner),title:'Modify Hash',onBack(){},onSubmitForm(){},onChangeField:change,onInvokeAction:invoke,onOpenActions(){}});
}
function edit(component,id,value){const node=id==='paths'?component.nodes('textarea')[0]:component.nodes('input').find(n=>n.props.id==='runtime-form-'+id+'FormCheckbox');assert(node,'actual field '+id);node.props.onChange({target:{value,checked:value}});component.render();}
// Checkbox DOM ids strip punctuation from their actual Form field key.
function checkbox(component,id,value){const node=component.nodes('input').find(n=>n.props.type==='checkbox'&&n.props.id.includes(id));assert(node);node.props.onChange({target:{checked:value}});component.render();}
function candidateRoot(component,requestId='select:1',revision=0,paths='["/synthetic/finder"]',acceptActionId='accept:1') {
 return {...component.props.root,props:{...component.props.root.props,selectionActionId:'select:2',selectionCandidate:{requestId,revision,paths,acceptActionId}}};
}
for(const edits of [[],['A','B','A'],['INVALID','["/synthetic/typed"]'],['PREFERENCE']]) {
 const sends=[],changes=[];const component=formMount((id,values)=>sends.push({id,values}),(id,value,values)=>changes.push({id,value,values}));
 if(edits[0]==='A')edit(component,'paths','A');
 const startingRevision=component.ref.current.getValues()._draftRevision;
 component.ref.current.invokeAction('select:1');assert.equal(sends[0].values._draftRevision,startingRevision);assert.equal(Object.keys(sends[0].values).length,8);
 component.ref.current.invokeAction('select:1');assert.equal(sends.length,1,'same pending action dispatched once');
 for(const value of edits){if(value==='PREFERENCE')checkbox(component,'showMD5Log',false);else edit(component,'paths',value);}
 const before=clone(component.ref.current.getValues());
 const response=candidateRoot(component,'select:1',startingRevision);component.render({...component.props,root:response});
 if(edits.length){assert.deepEqual(clone(component.ref.current.getValues()),before);assert.equal(sends.length,1,'edited candidate not admitted');}
 else {assert.equal(sends.length,2);assert.equal(sends[1].id,'accept:1');assert.equal(sends[1].values._draftRevision,1);assert.equal(sends[1].values.paths,'["/synthetic/finder"]');assert.equal(sends[1].values.showMD5Log,true);}
 const afterAdmission=clone(component.ref.current.getValues());
 component.render({...component.props,root:clone(response)});assert.equal(sends.length,edits.length?1:2,'candidate replay inert');
 // Both lower and equal-revision stale controlled fields are inert.
 for(const revision of [0,afterAdmission._draftRevision])component.render({...component.props,root:{...makeRoot(),props:{...makeRoot().props,draftRevision:revision}}});
 assert.deepEqual(clone(component.ref.current.getValues()),afterAdmission);
 checks.push('actual React pending selection '+(edits.join('/')||'unedited admission')+' and stale reconciliation');
}
{
 const sends=[];const component=formMount((id,values)=>sends.push({id,values}));
 checkbox(component,'showMD5Log',false);checkbox(component,'enableAudio',true);
 component.ref.current.invokeAction('select:1');const rev=component.ref.current.getValues()._draftRevision;
 component.render({...component.props,root:candidateRoot(component,'select:1',rev)});
 assert.equal(sends[1].values.showMD5Log,false);assert.equal(sends[1].values.enableAudio,true);assert.equal(sends[1].values._draftRevision,rev+1);
 component.ref.current.invokeAction('select:2');
 // A late first selection and its equal-ordinal server fields cannot consume
 // the newer request. Only the corresponding second candidate is admitted.
 component.render({...component.props,root:candidateRoot(component,'select:1',rev)});assert.equal(sends.length,3);
 component.render({...component.props,root:candidateRoot(component,'select:2',rev+1,'["/synthetic/second"]','accept:2')});assert.equal(sends.length,4);assert.equal(sends[3].id,'accept:2');
 component.render({...component.props,root:makeRoot('new-batch')});assert.equal(component.ref.current.getValues()._draftRevision,0);
 component.render({...component.props,root:candidateRoot(component,'select:2',rev+1)});assert.equal(sends.length,4);
 // A retired owner's late response is rejected too.
 component.render({...component.props,root:makeRoot('owned')});assert.equal(component.ref.current.getValues()._draftRevision,0);assert.equal(component.ref.current.getValues().paths,'[]');
 checks.push('actual React repeated explicit selection, preferences, new-batch and retired-owner fencing');
}
{
 const calls=[];const component=formMount((id,values)=>calls.push({id,values}));
 const surface=mount(path.join(patched,'index.tsx'),'ExtensionRuntimeSurface',{sessionId:'qa',extensionId:'qa',commandName:'modifyHash',root:makeRoot(),actions:[{id:'select:1',kind:'custom'}],title:'Modify Hash',onBack(){},onInvokeAction:(id,values)=>calls.push({id,values})},{
  './action-runtime':{Action:'Action',ActionPanel:'ActionPanel',ActionPanelOverlay:'Overlay'},'./action-runtime-registry':{ActionRegistryContext:{Provider:'Registry'}},'./form-runtime':{FormRuntime:'Form'},'./grid-runtime':{GridRuntime:'Grid'},'./list-runtime':{ListRuntime:'List'},'./hooks':{NavigationContext:{Provider:'Navigation'},setRuntimeContext(){}},
 });
 let formNode=surface.nodes('Form')[0];formNode.props.ref.current=component.ref.current;
 formNode.props.onSubmitForm(component.ref.current.getValues());assert.equal(calls.length,0,'Enter without effect action does not invoke selection');
 formNode.props.onOpenActions();surface.render();surface.nodes('Overlay')[0].props.onExecute({id:'select:1',kind:'custom'});
 assert.equal(calls.length,1);assert.equal(calls[0].values._draftRevision,0,'actual overlay starts owner before dispatch');
 formNode=surface.nodes('Form')[0];formNode.props.onChangeField('field','x',{paths:'x',_draftRevision:9});assert.equal(calls[1].values._draftRevision,9);
 checks.push('actual pinned surface overlay, selection snapshot and no submit fallback');
}
const rows=list.parseListRows({type:'List',children:[{type:'List.Section',props:{title:'Batch'},children:[{type:'List.Item',props:{id:'item',title:'雪',subtitle:'  committed ',actionIds:['owned',42]}}]}]});assert.equal(rows[0].subtitle,'committed');assert.equal(rows[0].actionIds.length,1);checks.push('pinned List parser action filtering');
assert.equal(detail.markdownFromNode({type:'Detail',props:{markdown:'    frozen result\n'}}),'    frozen result\n');checks.push('pinned Detail markdown parser');
// Execute the literal pinned/patched Finder producer against a closed synthetic
// Finder object. No osascript, PowerShell, clipboard, or real Finder is invoked.
const host=fs.readFileSync(path.join(V2,'evidence/host/src-tauri/src/native_core/extensions/process_host.rs'),'utf8');
const patchHost=fs.readFileSync(path.join(QA,'hostpatch/src-tauri/src/native_core/extensions/process_host.rs'),'utf8');
const extract=s=>{const m=s.match(/r#"(JSON\.stringify\(Application\('Finder'\)[^\n]+?)"#/);assert(m);return m[1];};
const selection=Array.from({length:129},(_,i)=>({url:()=>`file:///synthetic/${i}`}));
const old=JSON.parse(vm.runInNewContext(extract(host),{Application:n=>{assert.equal(n,'Finder');return {selection:()=>selection};}}));
const next=JSON.parse(vm.runInNewContext(extract(patchHost),{Application:()=>({selection:()=>selection})}));
assert.equal(old.length,128);assert.equal(next.length,129);assert(patchHost.includes('if values.len() > 128'));assert(patchHost.includes('metadata.is_dir()'));checks.push('actual Finder script removes silent truncation before host rejection; directory validation retained');
const result={checks,sourceHashes:Object.fromEntries(['form-runtime.tsx','list-runtime.tsx','detail-runtime.tsx','index.tsx'].map(n=>[n,crypto.createHash('sha256').update(fs.readFileSync(path.join(base,n))).digest('hex')])),windowsProducer:'extracted/static-reviewed; PowerShell execution not-run',gui:'not-run'};
async function processIntegration(binary) {
 const {spawn}=require('node:child_process'),readline=require('node:readline');
 const owned=fs.mkdtempSync(path.join(QA,'fixtures/renderer-'));
 for(const n of ['package','data'])fs.mkdirSync(path.join(owned,n));
 const child=spawn(binary,[],{cwd:QA,env:process.env,stdio:['pipe','pipe','pipe']});
 const responses=new Map(),inbox=[],receivers=[];let stderr='';
 child.stderr.on('data',b=>{stderr+=b;assert(stderr.length<1024*1024);});
 const send=v=>child.stdin.write(JSON.stringify(v)+'\n');
 const next=()=>inbox.length?Promise.resolve(inbox.shift()):new Promise(resolve=>receivers.push(resolve));
 readline.createInterface({input:child.stdout}).on('line',line=>{
  const v=JSON.parse(line);
  if(v.type==='response'){const pending=responses.get(v.id);assert(pending,'unexpected response');responses.delete(v.id);if(v.error)pending.reject(Error(JSON.stringify(v.error)));else pending.resolve(v.result);}
  else if(receivers.length)receivers.shift()(v);else inbox.push(v);
 });
 function rpc(method,params){const id='h:'+crypto.randomUUID();const promise=new Promise((resolve,reject)=>responses.set(id,{resolve,reject}));send({type:'request',id,method,params});promise.id=id;return promise;}
 const deadline=setTimeout(()=>{child.kill();throw Error('renderer/process timeout '+stderr);},20000);
 try {
  const hello=await next();assert.equal(hello.protocol,1);
  await rpc('initialize',{protocol:1,extensionId:hello.id,platform:'macOS',packageDir:path.join(owned,'package'),dataDir:path.join(owned,'data'),tempDir:path.join(owned,'unused-sdk-temp'),grantedCapabilities:['selection.read'],migrationPaths:{}});
  const mp4=Buffer.concat([Buffer.from([0,0,0,24]),Buffer.from('ftypisom'),Buffer.alloc(20)]),marker=Buffer.from('#$1024$#');
  for(const mode of ['untouched','paths-and-preference','A-B-A','invalid-valid','preference','repeated','error','cancel']) {
   const finder=path.join(owned,mode+'-finder'),typed=path.join(owned,mode+'-typed');fs.writeFileSync(finder,mp4);fs.writeFileSync(typed,mp4);
   const sid='renderer-'+mode;let latest=await rpc('session.open',{sessionId:sid,commandName:'modifyHash',preferences:{},argumentValues:{}}),component,requests=[];
   const invoke=(id,values)=>{
    const call=rpc('session.call',{sessionId:sid,operation:'extension:invoke-action',payload:{actionId:id,...(values?{formValues:values}:{})}});
    const applied=call.then(v=>{latest=v;if(v.root.type==='Form')component.render({...component.props,root:v.root});return v;});requests.push(applied);return applied;
   };
   component=mount(path.join(patched,'form-runtime.tsx'),'FormRuntime',{root:latest.root,title:'Modify Hash',onBack(){},onOpenActions(){},onInvokeAction:invoke,onChangeField:(id,value,snapshot)=>invoke(id,snapshot),onSubmitForm:values=>{const a=latest.actions.find(a=>a.kind==='submit-form');if(a)void invoke(a.id,values);}});
   const settle=async()=>{for(let i=0;i<5;i++){const batch=requests;requests=[];if(!batch.length)return;await Promise.all(batch);}throw Error('unbounded followup');};
   if(mode==='A-B-A'||mode==='invalid-valid'){edit(component,'paths',JSON.stringify([typed]));await settle();}
   component.ref.current.invokeAction(latest.actions.find(a=>a.kind==='custom').id);
   const reverse=await next();assert.equal(reverse.method,'host.selection.read');assert.deepEqual(reverse.params,{kind:'files'});
   if(mode==='paths-and-preference'){edit(component,'paths',JSON.stringify([typed]));checkbox(component,'showMD5Log',false);}
   if(mode==='A-B-A')for(const value of [JSON.stringify([typed]),'B',JSON.stringify([typed])])edit(component,'paths',value);
   if(mode==='invalid-valid')for(const value of ['INVALID',JSON.stringify([typed])])edit(component,'paths',value);
   if(mode==='preference')checkbox(component,'showMD5Log',false);
   const before=clone(component.ref.current.getValues());
   if(mode==='error')send({type:'response',id:reverse.id,error:{code:'synthetic',message:'selection unavailable'}});
   else if(mode==='cancel') {
    // The process suite tests protocol cancellation; here a reverse service's
    // explicit cancellation error drives actual React recovery.
    send({type:'response',id:reverse.id,error:{code:'cancelled',message:'synthetic chooser cancellation'}});
   } else send({type:'response',id:reverse.id,result:[finder]});
   await settle();
   if(['paths-and-preference','A-B-A','invalid-valid','preference','error','cancel'].includes(mode))assert.deepEqual(clone(component.ref.current.getValues()),before,'pending edit retained '+mode);
   else assert.equal(component.ref.current.getValues().paths,JSON.stringify([finder]));
   assert.deepEqual(fs.readFileSync(finder),mp4);assert.deepEqual(fs.readFileSync(typed),mp4,'selection/fields must be effect-free');
   if(mode==='repeated') {
    component.ref.current.invokeAction(latest.actions.find(a=>a.kind==='custom').id);const second=await next();send({type:'response',id:second.id,result:[typed]});await settle();assert.equal(component.ref.current.getValues().paths,JSON.stringify([typed]));
   }
   if(['preference','error','cancel'].includes(mode)){edit(component,'paths',JSON.stringify([typed]));await settle();}
   const draft=component.ref.current.getValues();assert(latest.actions.some(a=>a.kind==='submit-form'),'editable latest submit '+mode);
   // Execute the actual form's submit producer after complete candidate/field
   // admission. Only the visible latest paths may be changed.
   component.nodes('form')[0].props.onSubmit({preventDefault(){}});await settle();assert.equal(latest.root.type,'Detail');
   const chosen=JSON.parse(draft.paths)[0],other=chosen===typed?finder:typed;
   assert.deepEqual(fs.readFileSync(chosen),Buffer.concat([mp4,marker]));assert.deepEqual(fs.readFileSync(other),mp4);
   const outcome=JSON.parse(latest.root.props.markdown.split('\n').map(s=>s.slice(4)).join('\n'));
   if(draft.showMD5Log===false)assert.equal(outcome.results[0].old_md5,null);
   await rpc('session.dispose',{sessionId:sid});checks.push('extracted production + actual React '+mode+' ownership and latest submit');
  }
  await rpc('shutdown',{});child.stdin.end();await new Promise(resolve=>child.once('close',code=>{assert.equal(code,0);resolve();}));
  result.productionBinarySha256=crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex');
 } finally {clearTimeout(deadline);if(child.exitCode===null)child.kill();}
}
function finish(){result.patchedSourceHashes=Object.fromEntries(['form-runtime.tsx','index.tsx'].map(n=>[n,crypto.createHash('sha256').update(fs.readFileSync(path.join(patched,n))).digest('hex')]));result.adapterSourceSha256=crypto.createHash('sha256').update(fs.readFileSync(path.join(V2,'src/lib.rs'))).digest('hex');fs.writeFileSync(path.join(QA,'renderer-results.json'),JSON.stringify(result,null,2)+'\n');console.log('PASS pinned renderer/extracted-producer checks',checks.length);}
if(process.argv[2])processIntegration(path.resolve(process.argv[2])).then(finish).catch(e=>{console.error(e);process.exitCode=1;});else finish();
