"""Rebuild the unapplied prerequisite patch solely from pinned evidence copies."""
from pathlib import Path
import difflib
from qa_env import V2,QA
root=V2/'evidence/host'
changed={}
f='src/renderer/src/raycast-api/form-runtime.tsx';s=(root/f).read_text()
s=s.replace('onChangeField: (actionId: string, value: unknown) => Promise<void> | void','onChangeField: (actionId: string, value: unknown, snapshot?: Record<string, unknown>) => Promise<void> | void')
s=s.replace('  const localWheelCoordinates =', '''  // Refs fence ownership synchronously, before any dispatch/debounce/await.
  const completeDraft = root.props?.requiresDraftRevision === true
  const draftRef = useRef({ owner: String(root.props?.draftOwner ?? title),
    revision: Number(root.props?.draftRevision ?? 0), values,
    pending: null as { requestId: string; revision: number } | null })
  const retiredOwners = useRef(new Set<string>())
  const snapshot = (): Record<string, unknown> => completeDraft
    ? { ...draftRef.current.values, _draftRevision: draftRef.current.revision }
    : values
  const invokeFormAction = (actionId: string): void => {
    if (completeDraft && actionId === root.props?.selectionActionId) {
      if (draftRef.current.pending?.requestId === actionId) return
      draftRef.current = { ...draftRef.current,
        pending: { requestId: actionId, revision: draftRef.current.revision } }
      void onInvokeAction(actionId, snapshot())
    } else void onInvokeAction(actionId)
  }
  const localWheelCoordinates =''')
s=s.replace('    setValues((previous) => ({\n      ...reconcileFormValues(fields, previousFields ? previous : undefined, previousFields),\n      ...wheelDraft,\n    }))','''    if (completeDraft) {
      const owner = String(root.props?.draftOwner ?? title)
      if (retiredOwners.current.has(owner)) return
      if (owner !== draftRef.current.owner) {
        retiredOwners.current.add(draftRef.current.owner)
        draftRef.current = { owner, revision: Number(root.props?.draftRevision ?? 0),
          values: reconcileFormValues(fields), pending: null }
      }
      // Echoed fields never replace the authoritative local draft, even at
      // equal revision. Only an owned, one-use selection candidate can edit it.
      const admitted = admitFormSelection(draftRef.current, root.props?.selectionCandidate)
      draftRef.current = admitted.draft
      setValues(draftRef.current.values)
      if (admitted.actionId) void onInvokeAction(admitted.actionId, snapshot())
      return
    }
    setValues((previous) => ({
      ...reconcileFormValues(fields, previousFields ? previous : undefined, previousFields),
      ...wheelDraft,
    }))''')
s=s.replace('  }, [fields, nativeColorWheel, title])','  }, [fields, nativeColorWheel, title, completeDraft, root.props?.draftOwner, root.props?.selectionCandidate])')
s=s.replace('  getValues: () => Record<string, unknown>','  getValues: () => Record<string, unknown>\n  invokeAction: (actionId: string) => void')
s=s.replace('useImperativeHandle(ref, () => ({ getValues: () => values }), [values])','useImperativeHandle(ref, () => ({ getValues: snapshot, invokeAction: invokeFormAction }), [values, completeDraft, root])')
s=s.replace('''    setValues((previous) => ({ ...previous, [field.id]: value }))
    if (field.actionId) void onChangeField(field.actionId, value)''','''    if (completeDraft) {
      draftRef.current = advanceFormDraft(draftRef.current, field.id, value)
      setValues(draftRef.current.values)
      if (field.actionId) void onChangeField(field.actionId, value, snapshot())
    } else {
      setValues((previous) => ({ ...previous, [field.id]: value }))
      if (field.actionId) void onChangeField(field.actionId, value)
    }''')
s=s.replace('onSubmitForm(values)','onSubmitForm(snapshot())')
pos=s.index('type FormWheelCoordinates')
s=s[:pos]+'''type ProcessFormDraft = {
  owner: string; revision: number; values: Record<string, unknown>
  pending: { requestId: string; revision: number } | null
}

export function advanceFormDraft(
  previous: ProcessFormDraft, id: string, value: unknown
): ProcessFormDraft {
  if (!Number.isSafeInteger(previous.revision) || previous.revision >= Number.MAX_SAFE_INTEGER) {
    throw new Error('Form draft revision exhausted; reopen the command')
  }
  return { ...previous, revision: previous.revision + 1,
    values: { ...previous.values, [id]: value }, pending: null }
}

export function admitFormSelection(previous: ProcessFormDraft, value: unknown): {
  draft: ProcessFormDraft; actionId?: string
} {
  const candidate = value as { requestId?: unknown; revision?: unknown;
    paths?: unknown; acceptActionId?: unknown } | null
  if (!candidate || !previous.pending ||
      candidate.requestId !== previous.pending.requestId ||
      candidate.revision !== previous.pending.revision ||
      candidate.revision !== previous.revision || typeof candidate.paths !== 'string' ||
      typeof candidate.acceptActionId !== 'string') return { draft: previous }
  return { draft: advanceFormDraft(previous, 'paths', candidate.paths),
    actionId: candidate.acceptActionId }
}

'''+s[pos:]
changed[f]=s
f='src/renderer/src/raycast-api/index.tsx';s=(root/f).read_text().replace('onChangeField={(actionId, value) => onInvokeAction(actionId, { value })}','onChangeField={(actionId, value, snapshot) => onInvokeAction(actionId, snapshot ?? { value })}')
s=s.replace("|| actions.find((action) => action.kind === 'submit-form') || primaryAction", "|| actions.find((action) => action.kind === 'submit-form') || (root.props?.requiresDraftRevision ? undefined : primaryAction)")
s=s.replace('[actions, kind, primaryAction, root.props?.nativeColorWheel]', '[actions, kind, primaryAction, root.props?.nativeColorWheel, root.props?.requiresDraftRevision]')
s=s.replace('                void onInvokeAction(action.id)', '''                if (kind === 'form' && root.props?.requiresDraftRevision) {
                  formRuntimeRef.current?.invokeAction(action.id)
                  return
                }
                void onInvokeAction(action.id)''')
changed[f]=s
f='src-tauri/src/native_core/extensions/process_host.rs';s=(root/f).read_text().replace("selection().slice(0,128).map", "selection().map").replace(' | Select-Object -First 128','').replace('if path.is_absolute() && path.is_file() {\n                files.push(path);\n            }','''if !path.is_absolute() {
                return Err("Selection path must be absolute".into());
            }
            let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
                return Err("Selection contains a link or special file".into());
            }
            files.push(path);''');changed[f]=s
patch=''
for f,s in changed.items():
    before=(root/f).read_text();assert s!=before,f
    out=QA/'hostpatch'/f;out.parent.mkdir(parents=True,exist_ok=True);out.write_text(s)
    patch+=''.join(difflib.unified_diff(before.splitlines(True),s.splitlines(True),fromfile='a/'+f,tofile='b/'+f))
(V2/'host-prerequisites.patch').write_text(patch)
print('unapplied prerequisite patch generated for',len(changed),'pinned files')
