from qa_env import *
import json
v1=json.loads((V2.parent/'package.json').read_text())
m={'schemaVersion':3,'id':'com.tezbar.native.modify-hash','slug':'modify-hash','name':'Modify Hash','version':'0.2.0','description':'Modify or restore media hashes and compress or extract portable ZIP archives.','icon':'resources/hasher.png','runtime':'rust','execution':'process','protocolVersion':1,'repository':v1['tezbar']['repository'],'author':v1['author'],'platforms':['macOS','Windows'],'commands':[{**c,'icon':'resources/hasher.png','runtime':'rust','refreshMs':None,'concurrency':'serialize','timeoutMs':300000} for c in v1['commands']],'preferences':v1['preferences'],'replaces':['com.tezbar.modify-hash','raycast.modify-hash'],'capabilities':['selection.read'],'artifacts':{},'verification':[{'platform':p,'state':'not-run','note':'Real OS/GUI acceptance not run. Host prerequisites and catalog/acquisition remain unapplied.'}for p in ['macOS','Windows']]}
(V2/'extension.json').write_text(json.dumps(m,indent=2)+'\n')
(V2/'LICENSE').write_bytes((V2.parent/'LICENSE').read_bytes())
