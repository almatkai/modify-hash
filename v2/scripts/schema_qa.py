"""Evaluate all assertion keywords used by the unchanged pinned schema.
Not a general JSON Schema metaschema validator; unsupported keywords fail closed.
"""
from qa_env import *
import copy,json,re,urllib.parse
schema=json.loads((V2/'evidence/host/schemas/extension-v3.schema.json').read_text())
allowed={'$schema','$id','$ref','$defs','title','description','type','required','properties','additionalProperties','propertyNames','pattern','const','enum','minItems','maxItems','uniqueItems','items','minimum','maximum','minLength','maxLength','maxProperties','format','allOf'}
def audit(s):
    if isinstance(s,bool):return
    assert not set(s)-allowed,set(s)-allowed
    for k in ['properties','$defs']:
        for sub in s.get(k,{}).values():audit(sub)
    for k in ['additionalProperties','propertyNames','items']:
        if k in s and isinstance(s[k],(dict,bool)):audit(s[k])
    for sub in s.get('allOf',[]):audit(sub)
audit(schema)
def evaluate(s,v):
    if isinstance(s,bool):return s
    if '$ref'in s:
        assert s['$ref'].startswith('#/')
        ref=schema
        for k in s['$ref'][2:].split('/'):ref=ref[k.replace('~1','/').replace('~0','~')]
        if not evaluate(ref,v):return False
    typ=s.get('type');kinds=typ if isinstance(typ,list)else[typ] if typ else[]
    checks={'object':lambda:isinstance(v,dict),'array':lambda:isinstance(v,list),'string':lambda:isinstance(v,str),'integer':lambda:isinstance(v,int)and not isinstance(v,bool),'number':lambda:isinstance(v,(int,float))and not isinstance(v,bool),'boolean':lambda:isinstance(v,bool),'null':lambda:v is None}
    if kinds and not any(checks[t]()for t in kinds):return False
    if 'const'in s and v!=s['const']:return False
    if 'enum'in s and v not in s['enum']:return False
    if isinstance(v,dict):
        if not all(k in v for k in s.get('required',[])):return False
        if len(v)>s.get('maxProperties',float('inf')):return False
        for k,value in v.items():
            if 'propertyNames'in s and not evaluate(s['propertyNames'],k):return False
            rule=s.get('properties',{}).get(k,s.get('additionalProperties',True))
            if not evaluate(rule,value):return False
    if isinstance(v,list):
        if not s.get('minItems',0)<=len(v)<=s.get('maxItems',float('inf')):return False
        if s.get('uniqueItems')and len({json.dumps(x,sort_keys=True)for x in v})!=len(v):return False
        if 'items'in s and not all(evaluate(s['items'],x)for x in v):return False
    if isinstance(v,str):
        if not s.get('minLength',0)<=len(v)<=s.get('maxLength',float('inf')):return False
        if 'pattern'in s and re.search(s['pattern'],v)is None:return False
        if s.get('format')=='uri'and not urllib.parse.urlsplit(v).scheme:return False
    if isinstance(v,(int,float))and not isinstance(v,bool):
        if not s.get('minimum',-float('inf'))<=v<=s.get('maximum',float('inf')):return False
    return all(evaluate(sub,v)for sub in s.get('allOf',[]))
def main():
    m=json.loads((V2/'extension.json').read_text());assert evaluate(schema,m)
    mutations=[lambda x:x.update(schemaVersion=2),lambda x:x.update(protocolVersion=2),lambda x:x.update(execution='tauri'),lambda x:x.update(runtime='javascript'),lambda x:x.update(id='wrong.id'),lambda x:x.update(commands=[]),lambda x:x['commands'][0].update(runtime='stub'),lambda x:x['commands'][0].update(refreshMs=0),lambda x:x['commands'][0].update(timeoutMs=999999),lambda x:x.update(capabilities=['filesystem.any']),lambda x:x.update(artifacts={'fake':{}}),lambda x:x.update(artifacts={'aarch64-apple-darwin':{'url':'http://example.com/x','sha256':'a'*64,'executable':'bin/app','executableSha256':'a'*64}}),lambda x:x.update(files={'../escape':'a'*64}),lambda x:x.update(replaces=['unknown']),lambda x:x.pop('preferences'),lambda x:x['verification'][0].update(state='pretend')]
    for i,change in enumerate(mutations):
        invalid=copy.deepcopy(m);change(invalid);assert not evaluate(schema,invalid),i
    (QA/'schema-results.json').write_text(json.dumps({'valid':1,'negativeCases':len(mutations),'validator':'stdlib evaluator of all assertion keywords used by unchanged pinned schema; no general metaschema certification'},indent=2)+'\n');print('PASS pinned schema and',len(mutations),'negative cases')
if __name__=='__main__':main()
