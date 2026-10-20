import { ExternalLink, Trash2 } from 'lucide-react';

interface Attachment {
  name: string;
  path: string;
  size?: number;
}

interface AttachmentListProps {
  attachments: Attachment[];
  onRemove?: (index: number) => void;
  readonly?: boolean;
}

export default function AttachmentList({ attachments, onRemove, readonly = false }: AttachmentListProps) {
  if (attachments.length === 0) {
    return (
      <div style={{ color: 'var(--color-text-tertiary)', fontSize: '13px' }} className="py-2">
        暂无附件
      </div>
    );
  }

  function formatFileSize(bytes?: number) {
    if (!bytes) return "";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function getFileExtension(filename: string) {
    return filename.split('.').pop()?.toLowerCase() || '';
  }

  function getFileIcon(ext: string) {
    switch (ext) {
      case 'pdf':
        return '📄';
      case 'doc':
      case 'docx':
        return '📝';
      case 'xls':
      case 'xlsx':
        return '📊';
      case 'ppt':
      case 'pptx':
        return '📑';
      case 'jpg':
      case 'jpeg':
      case 'png':
      case 'gif':
        return '🖼️';
      case 'zip':
      case 'rar':
        return '📦';
      default:
        return '📎';
    }
  }

  return (
    <div className="space-y-2">
      {attachments.map((attachment, index) => (
        <div
          key={index}
          className="flex items-center justify-between p-2 rounded-lg transition-colors"
          style={{ background: 'var(--color-bg-subtle)' }}
          onMouseEnter={(e) => e.currentTarget.style.background = 'var(--color-bg-muted)'}
          onMouseLeave={(e) => e.currentTarget.style.background = 'var(--color-bg-subtle)'}
        >
          <div className="flex items-center gap-2">
            <span className="text-lg">{getFileIcon(getFileExtension(attachment.name))}</span>
            <div>
              <p className="text-sm font-medium" style={{ color: 'var(--color-text)' }}>{attachment.name}</p>
              {attachment.size && (
                <p style={{ color: 'var(--color-text-tertiary)', fontSize: '11px' }}>{formatFileSize(attachment.size)}</p>
              )}
            </div>
          </div>
          <div className="flex items-center gap-1">
            <a
              href={attachment.path}
              target="_blank"
              rel="noopener noreferrer"
              className="p-1.5 rounded-lg transition-colors"
              style={{ color: 'var(--color-text-tertiary)' }}
              onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--color-primary)'; e.currentTarget.style.background = 'var(--color-primary-light)'; }}
              onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--color-text-tertiary)'; e.currentTarget.style.background = 'transparent'; }}
              title="打开"
            >
              <ExternalLink className="w-4 h-4" />
            </a>
            {!readonly && onRemove && (
              <button
                onClick={() => onRemove(index)}
                className="p-1.5 rounded-lg transition-colors"
                style={{ color: 'var(--color-text-tertiary)' }}
                onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--color-danger)'; e.currentTarget.style.background = 'var(--color-danger-light)'; }}
                onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--color-text-tertiary)'; e.currentTarget.style.background = 'transparent'; }}
                title="移除"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
