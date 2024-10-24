resource "azurerm_user_assigned_identity" "loyalty_app_identity" {
  location            = azurerm_resource_group.modern_apps_container_apps.location
  name                = "LoyaltyAppIdentity-${var.env}"
  resource_group_name = azurerm_resource_group.modern_apps_container_apps.name
}