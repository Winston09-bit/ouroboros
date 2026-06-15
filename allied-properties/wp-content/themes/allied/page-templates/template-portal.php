<?php
/**
 * Template Name: Portal (Gated)
 *
 * Assign to each portal page created at /portals/builder/, /portals/investor/,
 * and /portals/partner/. The template reads the page slug to determine which
 * portal it is and gates access to the matching role. Portal body content
 * (documents, lists, PDFs) is edited in the page editor and only shown to
 * authorised, signed-in users.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();

while ( have_posts() ) :
	the_post();

	$slug    = get_post_field( 'post_name', get_the_ID() );
	$portals = allied_portals();
	// Fall back to 'partner' gating if the slug isn't one of the known portals.
	if ( ! isset( $portals[ $slug ] ) ) {
		$slug = 'partner';
	}
	$label = $portals[ $slug ];
	?>
	<section class="page-banner">
		<div class="container">
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Secure Portal', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
		</div>
	</section>

	<section class="section">
		<div class="container">
			<?php
			// Gate. Renders login / access-denied and returns false when blocked.
			if ( allied_portal_gate( $slug, $label ) ) :
				?>
				<div style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:var(--space-sm);margin-bottom:var(--space-lg);">
					<p class="text-muted" style="margin:0;"><?php printf( esc_html__( 'Signed in as %s.', 'allied' ), esc_html( wp_get_current_user()->display_name ) ); ?></p>
					<a class="btn btn--ghost" href="<?php echo esc_url( wp_logout_url( home_url( '/portals/' ) ) ); ?>"><?php esc_html_e( 'Sign out', 'allied' ); ?></a>
				</div>

				<div class="split" style="align-items:start;">
					<div class="entry-content stack">
						<?php
						if ( get_the_content() ) {
							the_content();
						} else {
							echo '<p class="notice notice--info">' . esc_html__( 'Editor: add this portal\'s documents and updates in the page content. You can use the File block for PDFs, or the document list pattern below.', 'allied' ) . '</p>';
						}
						?>
					</div>

					<aside>
						<h3 style="font-family:var(--font-body);font-size:var(--fs-h4);"><?php esc_html_e( 'Documents', 'allied' ); ?></h3>
						<?php
						// Show PDFs/files attached to this portal page as a clean list.
						$files = get_posts(
							array(
								'post_type'      => 'attachment',
								'post_parent'    => get_the_ID(),
								'posts_per_page' => -1,
								'post_status'    => 'inherit',
							)
						);
						if ( $files ) :
							echo '<ul class="doc-list">';
							foreach ( $files as $file ) {
								$type = strtoupper( pathinfo( get_attached_file( $file->ID ), PATHINFO_EXTENSION ) );
								printf(
									'<li><a href="%s" target="_blank" rel="noopener"><span>%s</span><span class="doc-type">%s</span></a></li>',
									esc_url( wp_get_attachment_url( $file->ID ) ),
									esc_html( $file->post_title ),
									esc_html( $type )
								);
							}
							echo '</ul>';
						else :
							echo '<p class="text-muted">' . esc_html__( 'No documents uploaded yet. Attach files to this page in the Media library to list them here.', 'allied' ) . '</p>';
						endif;
						?>
					</aside>
				</div>
			<?php endif; ?>
		</div>
	</section>
	<?php
endwhile;

get_footer();
