output "stagecraft_identity_client_id" { value = azurerm_user_assigned_identity.stagecraft.client_id }
output "deployd_identity_client_id" { value = azurerm_user_assigned_identity.deployd.client_id }

output "stagecraft_serviceaccount_name" { value = "stagecraft-api-sa" }
output "deployd_serviceaccount_name" { value = "deployd-api-sa" }
