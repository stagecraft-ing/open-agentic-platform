{{/*
Expand the name of the chart.
*/}}
{{- define "tenant-hello.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a default fully qualified app name. Stagecraft overrides
.Values.fullnameOverride at deploy time with `<env-slug>-<project-slug>`
or similar; default falls back to chart name + release name.
*/}}
{{- define "tenant-hello.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{/*
Common labels shared across every rendered object.
*/}}
{{- define "tenant-hello.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{ include "tenant-hello.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
oap.spec: "136-tenant-hello-demo-service"
{{- end -}}

{{/*
Selector labels — must be a stable subset of full labels.
*/}}
{{- define "tenant-hello.selectorLabels" -}}
app.kubernetes.io/name: {{ include "tenant-hello.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
ServiceAccount name — chart-managed when serviceAccount.create == true,
otherwise the operator-supplied value.
*/}}
{{- define "tenant-hello.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "tenant-hello.fullname" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}
