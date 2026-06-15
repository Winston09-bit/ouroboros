<?php
/**
 * Header.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
?><!DOCTYPE html>
<html <?php language_attributes(); ?>>
<head>
	<meta charset="<?php bloginfo( 'charset' ); ?>" />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
	<link rel="profile" href="https://gmpg.org/xfn/11" />
	<?php wp_head(); ?>
</head>

<body <?php body_class(); ?>>
<?php wp_body_open(); ?>

<a class="skip-link screen-reader-text" href="#main"><?php esc_html_e( 'Skip to content', 'allied' ); ?></a>

<header class="site-header">
	<div class="container site-header__inner">
		<?php if ( has_custom_logo() ) : ?>
			<div class="site-logo"><?php the_custom_logo(); ?></div>
		<?php else : ?>
			<a class="site-brand" href="<?php echo esc_url( home_url( '/' ) ); ?>" rel="home">
				<svg class="site-brand__mark" viewBox="0 0 40 40" aria-hidden="true" focusable="false">
					<rect x="2" y="2" width="36" height="36" fill="none" stroke="currentColor" stroke-width="1.5"/>
					<path d="M11 28 L20 11 L29 28 Z" fill="none" stroke="currentColor" stroke-width="1.5"/>
				</svg>
				<span><strong><?php bloginfo( 'name' ); ?></strong></span>
			</a>
		<?php endif; ?>

		<button class="nav-toggle" aria-expanded="false" aria-controls="primary-nav">
			<span class="screen-reader-text"><?php esc_html_e( 'Toggle menu', 'allied' ); ?></span>
			<span aria-hidden="true">☰</span> <?php esc_html_e( 'Menu', 'allied' ); ?>
		</button>

		<nav id="primary-nav" class="primary-nav" aria-label="<?php esc_attr_e( 'Primary', 'allied' ); ?>">
			<?php
			if ( has_nav_menu( 'primary' ) ) {
				wp_nav_menu(
					array(
						'theme_location' => 'primary',
						'container'      => false,
						'depth'          => 2,
					)
				);
			} else {
				allied_default_menu();
			}
			?>
		</nav>

		<div class="header-actions">
			<a class="btn btn--ghost" href="<?php echo esc_url( home_url( '/portals/' ) ); ?>"><?php esc_html_e( 'Portals', 'allied' ); ?></a>
		</div>
	</div>
</header>

<main id="main" tabindex="-1">
